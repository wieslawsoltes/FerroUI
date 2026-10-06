# Browser platform: how to target the browser from FerroUI

Status: design verified by the feasibility spike of 2026-10-05 (section 13); implementation in progress. Upstream reference: Avalonia `main` at `17350180c3` (2026-10-01), `src/Browser/Avalonia.Browser` (86 tracked files, about 7,000 lines of C# and TypeScript) and `samples/ControlCatalog.Browser`. External facts were checked on 2026-10-04; each is tagged **[V]** (verified against the crate, repository or docs), **[M]** (measured on 2026-10-05 in the feasibility spike, see section 13) or **[U]** (not verified, see section 13).

## 1. Recommendation summary

Build the browser backend as `src/Browser/FerroUI.Browser`, a single-threaded backend that mirrors upstream file for file. Ship it first on **`wasm32-unknown-emscripten` with the existing Skia and HarfBuzz backends** (Skia Ganesh on WebGL2, Skia raster to a 2D canvas as fallback). That is the only target where today's mandatory native dependencies build, it is what upstream does, and it gives pixel parity with desktop without waiting for a second renderer. Add **`wasm32-unknown-unknown` with Vello plus a pure-Rust shaper as a second configuration** once the Vello backend exists on desktop; nothing in the browser crate may depend on Emscripten outside one GL-context module. Port upstream's TypeScript modules nearly 1:1 and bind them through a flat, narrow import/export boundary (`wasm-bindgen` extern blocks, no `web-sys` types outside `interop/`). Run everything on the browser main thread: the dispatcher is driven by posted tasks, rendering by `requestAnimationFrame`, and the application start-up function returns instead of running a loop. Do not support WASM threads. Use the single-view lifetime with one top-level per host `<div>` and overlay popups. Ship an embedded default font; fonts are otherwise registered at run time from bytes. Storage, native control host, service worker and accessibility come after the MVP. The core must, starting now, avoid blocking loops, blocking waits, `std::fs`, `std::thread` and `std::time::Instant` on shared paths, and must expose asynchronous platform services as completion callbacks.

## 2. What upstream does (facts that drive the design)

| Area | Upstream behaviour |
|---|---|
| Runtime | .NET on Emscripten. Skia and HarfBuzz come as prebuilt Emscripten static libraries (`SkiaSharp.NativeAssets.WebAssembly`, `HarfBuzzSharp.NativeAssets.WebAssembly`) linked into the app by the .NET WASM native build (`WasmBuildNative=true`). |
| Boundary | 106 `[JSImport]` and 22 `[JSExport]` declarations. JS objects are opaque `JSObject` handles. JS reaches managed code through `getAssemblyExports("Avalonia.Browser.dll")`. |
| JS bundles | esbuild (run through bun) produces three ES modules: `avalonia.js` (main), `storage.js` (lazy, pulls in the `native-file-system-adapter` polyfill), `avalonia-sw.js` (service worker). |
| Bootstrap | `StartBrowserAppAsync("out")`: import the JS module, register the platform, `SetupWithLifetime(BrowserSingleViewLifetime)`, create an `AvaloniaView` on the `<div>`. In single-threaded mode **nothing calls a run loop**; `Main` returns. |
| Windowing | `IWindowingPlatform.CreateWindow` throws. Only `ITopLevelImpl` exists, wrapped in an `EmbeddableControlRoot`. `CreatePopup()` returns `null`, so every popup is an overlay popup inside the same top-level. Several views on one page are supported (one per host element, looked up by integer top-level id). |
| DOM per view | Host `<div>` (`tabIndex=0`, `touch-action:none`, context menu suppressed) containing a canvas (created lazily), a native-control host `<div>`, and a hidden `<input type=text>` for IME. |
| Rendering | Mode list `WebGL2, WebGL1, Software2D` with fallback. WebGL: the JS side creates the context and registers it with Emscripten's `GL` object; Skia then builds its native GL interface. Software: Skia raster into a retained RGBA8888 buffer, blitted with `putImageData` from a view over WASM memory. Canvas pixel size is set at draw time, not in the resize callback. |
| Size/DPI | One shared `ResizeObserver`, using `devicePixelContentBoxSize` when available, reporting device pixels plus `devicePixelRatio`. |
| Dispatcher | `BrowserSingleThreadedDispatcherImpl`: `Signal` posts a task (`scheduler.postTask` "user-blocking", else `MessageChannel`, else `setTimeout(0)`), background processing posts at "user-visible", timers use one `setTimeout`. No pending-input query, no nested loop. |
| Render timer | A perpetual `requestAnimationFrame` loop. One shared render loop and one compositor per surface, running on the UI thread. |
| Threads | Optional (`WasmEnableThreads`): UI thread on a worker with the managed blocking dispatcher, a separate render worker with `OffscreenCanvas`, raw input grouped and queued across threads. |
| Input | Pointer events on the host element, `wheel`, key events, composition events on the hidden input, drag events, `navigator.virtualKeyboard` geometry. |
| Services | Clipboard (async Clipboard API with a paste-event fallback), storage provider (File System Access API or polyfill, bookmarks in IndexedDB), launcher (`window.open`), screens (`window.screen`, `getScreenDetails`), insets (CSS `env(safe-area-inset-*)`, fullscreen), platform settings (`prefers-color-scheme`, `prefers-contrast`, `navigator.language`), activation (`visibilitychange`), back navigation (`history.pushState`/`popstate`), CSS cursors, DOM native control host. |
| Not present | No `Avalonia.Browser.Blazor` project in this checkout. No accessibility bridge. No window icon (stub loader). |

## 3. Target triple (question 1)

**Decision: `wasm32-unknown-emscripten` first; `wasm32-unknown-unknown` as a second configuration tied to the Vello backend.**

| Dependency | `wasm32-unknown-emscripten` | `wasm32-unknown-unknown` |
|---|---|---|
| `skia-safe` 0.153.x | Supported; needs emsdk 5.0 or newer, `EMSDK` set, `EMCC_CFLAGS="-s ERROR_ON_UNDEFINED_SYMBOLS=0"`, `MAX_WEBGL_VERSION=2` for WebGL. Listed among targets with prebuilt binaries. **[V]** (rust-skia README). The `gl` feature resolves to the prebuilt binary `wasm32-unknown-emscripten-ganesh-gl-jpegd-jpege-pdf` (`libskia.a` 13.4 MB): Ganesh with the WebGL interface, raster, FreeType with the empty custom font manager, PNG/JPEG/GIF/BMP/ICO decoders, PNG and JPEG encoders, path ops, image filters, SVG; no WebP, no text layout module, no ICU. That covers everything `FerroUI.Skia` uses. It must be linked with `em++`. **[M]** | Explicitly unsupported by rust-skia ("fundamentally incompatible with linking C code"). **[V]** |
| Skia GPU in the browser | Ganesh over WebGL (the `gl` feature). **[V]** that the README documents WebGL; drawing through a WebGL2 context registered with Emscripten's `GL` object works **[M]** (Chrome). Graphite on this target: no prebuilt binary carries it, and the build falls back to compiling Skia from source, which fails at dependency sync **[M]**; unavailable. | n/a |
| `harfbuzz-sys` 0.8 `bundled` | Build script compiles `harfbuzz/src/harfbuzz.cc` with the `cc` crate, with no WASM-specific handling. **[V]** (build.rs). Builds under `em++` and runs (HarfBuzz 8.4.0). **[M]** | No C++ toolchain or libc for this target through `cc`; would need wasi-sdk plus manual sysroot work. Treat as unsupported. **[U]** (not attempted) |
| `wasm-bindgen` / `web-sys` | Supported since 0.2.115 (March 2026) with Emscripten 6.0.10 or newer; binary crate required; documented as "still being smoothed out". **[V]** (wasm-bindgen guide). Works with crate and command-line tool 0.2.129 on Emscripten 6.0.10: `-sWASM_BINDGEN` makes `emcc` run the `wasm-bindgen` executable from `PATH`, whose version must equal the crate's. Module-file imports, strings, byte slices, opaque `JsValue` handles and exports were exercised. `-Cpanic=abort`, which the guide asks for, does **not** build with stable Rust 1.90 on this target (`the crate core requires panic strategy unwind`); the default `unwind` works, including `catch_unwind`. `-sINVOKE_RUN=0` is needed, otherwise `main` runs twice. **[M]** | First-class. **[V]** |
| `wgpu` | Not working today: issue #10274 (open) and PR #10515 (open, 2026-10-02) that routes Emscripten through the web-sys backend. **[V]** | First-class WebGPU and WebGL2 backends. **[V]** (general knowledge of the crate; versions not re-checked) |
| `vello` (classic) | Blocked on `wgpu` above. | Works on WebGPU only (compute shaders). **[V]** |
| `vello_gpu` (was `vello_hybrid`) / `vello_cpu` | **[U]** | `vello_gpu` has a native WebGL2 backend and a wgpu backend without compute shaders; `vello_cpu` is described as the most mature. **[V]** (Vello README). MSRV 1.89. **[V]** |
| `harfrust` 0.14, `skrifa` | Pure Rust, expected to build. **[U]** | Pure Rust. `harfrust` tracks HarfBuzz 14.5.1; `rustybuzz` was archived 2026-07-26 and is flagged by RUSTSEC-2026-0206, so it is not an option. **[V]** |
| `tiny-skia` | Builds, but adds nothing over Skia raster here. | Possible software fallback, but `vello_cpu` covers the same need with one renderer family. Not recommended. |

Reasons for the order:

1. FerroUI's renderer and shaper today are Skia and HarfBuzz. Only Emscripten builds them. Choosing `wasm32-unknown-unknown` first would make the browser port wait for a renderer that does not exist yet, and would give up render-test parity with desktop.
2. `wasm32-unknown-unknown` has the better toolchain (trunk, mature `wasm-bindgen`, `wgpu`). It becomes worthwhile exactly when the Vello backend and a pure-Rust shaper land, and those are useful on desktop too.
3. Both targets can share one browser crate because `wasm-bindgen` now covers both. This was the one new and still-experimental piece of the plan; the spike confirmed it **[M]**, so the fallback of section 5 is not needed.

Toolchain consequences: Emscripten 6.0.10 or newer is the floor (set by `wasm-bindgen`, above rust-skia's 5.0 floor). rust-skia's prebuilt binaries link cleanly under emsdk 6.0.10 **[M]**, so Skia is never built from source: CI downloads a 15 MB archive. The panic strategy stays `unwind` (the only one that builds with the pinned toolchain), so the `catch_unwind` uses of the core keep working in the browser.

Pinned toolchain **[M]**: Emscripten 6.0.10, Rust 1.90.0 with the `wasm32-unknown-emscripten` target, `skia-safe` 0.153.3, `harfbuzz-sys` 0.8.0, `wasm-bindgen` crate and command-line tool 0.2.129.

## 4. Renderer (question 2)

**Decision: both, behind the existing platform render contract; Skia on Emscripten is the first and reference renderer, Vello is second and arrives with the `wasm32-unknown-unknown` configuration.**

| Criterion | Skia on Emscripten (Ganesh/WebGL2, raster fallback) | Vello on WebGPU, `vello_gpu` on WebGL2, `vello_cpu` fallback |
|---|---|---|
| Availability to FerroUI | Backend exists (`FerroUI.Skia`) | Backend planned, not written |
| Coverage of the drawing-context contract | Complete: path ops, stroking and widening, hit testing, image filters, blur and box shadow, blend modes, opacity masks, layers, bitmap decode/encode, render-target bitmaps | Fills, strokes, gradients, images, clips, blends are covered. Path boolean ops, geometry widening and containment tests, blur and filter effects, bitmap codecs and readback need extra crates or own code (`kurbo`, `image`). Per-feature status **[U]** |
| Text quality | Same rasteriser as desktop, hinting and LCD/greyscale options | Outline rendering, no hinting by default; small UI text is visibly softer. **[U]** for the current release |
| Binary size | Large. Skia (Ganesh and raster) with HarfBuzz and one embedded font links to 4.6 MB of WASM, 1.9 MB with gzip, before any framework code **[M]** | Smaller, pure Rust, benefits from LTO and `wasm-opt`. **[U]**, measure |
| Start-up | Synchronous context creation | WebGPU adapter and device requests are asynchronous; start-up needs a continuation |
| Reach | WebGL2 is available everywhere; raster fallback always works | WebGPU: Chrome/Edge desktop, Safari 26, Firefox 141+ on Windows and 145+ on Apple Silicon; Firefox on Linux and Android not shipped, Chrome on Linux limited. **[U]** (secondary sources). So the WebGL2 path is not optional |
| API stability | Stable | Churning: `vello_hybrid` was renamed `vello_gpu` in August 2026 **[U]** (secondary source); README says it is "intended to become the primary renderer" |

Constraints this puts on `FerroUI.Skia`:

- The browser needs **Ganesh on GL**. Desktop enables only `graphite` and `metal`. The GPU abstraction in `FerroUI.Skia` (upstream `ISkiaGpu`, `GlSkiaGpu`, `IGlPlatformSurface`) stays generic over Ganesh and Graphite, and the Skia features are selected per target in the manifest of `FerroUI.Skia`. No published Skia binary carries Graphite and Ganesh together (checked for macOS: the `graphite`+`metal`+`gl` combination does not exist **[M]**), so the Ganesh files of `FerroUI.Skia` compile only where the build has Ganesh (the browser); the OpenGL contracts (`FerroUI.OpenGL`) are free of such conditions and are tested on desktop.
- A framebuffer surface path (upstream `IFramebufferPlatformSurface`, `RetainedFramebuffer`) is required for the raster fallback.
- A "graphics not ready yet" state is required (upstream `IPlatformGraphicsReadyStateFeature`): the compositor may exist before the render target does.

Browser rendering modes mirror upstream: `BrowserRenderingMode { Software2D = 1, WebGL1, WebGL2 }`, default order WebGL2, WebGL1, Software2D, with `failIfMajorPerformanceCaveat: true` so that software GL falls through to the raster path. Add `WebGpu` only with the Vello configuration.

## 5. JS interop shape (question 3)

**Decision: a mix that leans on upstream. Port the TypeScript modules nearly 1:1 and keep the boundary flat and narrow; bind it with `wasm-bindgen`; do not spread `web-sys` through the Rust code.**

Why not `web-sys` everywhere: the upstream TypeScript is about 2,000 lines of debugged browser quirks (caret mirror for IME positioning, clipboard fallbacks, resize rounding, file-picker polyfill, service-worker streaming). Rewriting it in `web-sys` is slower to write, harder to diff against upstream, and larger in generated glue. Why not hand-written `extern "C"` plus an Emscripten JS library: it works only on Emscripten and needs manual string and handle marshalling.

Rules:

1. One Rust file per upstream `Interop/*.cs` under `interop/`, containing only `#[wasm_bindgen(module = ...)] extern` blocks (imports) and `#[wasm_bindgen]` free functions (exports). JS objects are held as opaque `JsValue` (replaces `JSObject`).
2. Boundary types are limited to numbers, booleans, strings, byte slices, number arrays, opaque handles, and for asynchronous calls a request id plus a completion export. No Rust closures passed to JS except through that completion mechanism.
3. No `web-sys` or `js-sys` type appears outside `interop/` and the two render-target files. This is what makes the fallback mechanical.
4. JS-to-Rust calls are synchronous and return their result directly. Upstream returns `Task<bool>` from key handlers and calls `preventDefault` in a `.then`; do not copy that (see section 14).

**Fallback** (not needed: `wasm-bindgen` on Emscripten works, section 13), had `wasm-bindgen` on Emscripten proved unusable: keep the TypeScript unchanged, replace `interop/*.rs` bindings with `extern "C"` imports resolved by an Emscripten `--js-library` and an integer handle table on the JS side. Rule 2 keeps this a contained change.

| Upstream JS module | Purpose | FerroUI counterpart |
|---|---|---|
| `avalonia.ts` | Main entry, re-exports, service-worker registration, module URL helpers | `webapp/modules/ferroui.ts` (URL helpers only if assets are fingerprinted) |
| `avalonia/jsExports.ts` | Resolves managed exports | `ferroui/ferroExports.ts`: imports the `wasm-bindgen` exports; much smaller |
| `avalonia/dom.ts` | Host creation, canvas creation, global DOM events, safe area, dark mode, language, fullscreen | `ferroui/dom.ts`, 1:1 |
| `avalonia/input.ts` | All input subscriptions, clipboard, cursor, IME element helpers, pointer capture | `ferroui/input.ts`, 1:1 with the fixes in section 14 |
| `avalonia/caretHelper.ts` | Caret pixel position inside the hidden input | `ferroui/caretHelper.ts`, 1:1 |
| `avalonia/singleThreadedDispatcher.ts` | `signal`, background request, one-shot timer | `ferroui/singleThreadedDispatcher.ts`, 1:1 |
| `avalonia/timer.ts` | `requestAnimationFrame` loop | `ferroui/timer.ts`, pass the timestamp |
| `avalonia/rendering/canvasSurface.ts`, `resizeHandler.ts`, `webRenderTarget.ts`, `webRenderTargetRegistry.ts`, `renderingMode.ts` | Canvas surface, size observation, target registry | Same names under `ferroui/rendering/`, without the worker branch |
| `avalonia/rendering/webGlRenderTarget.ts` | Creates the WebGL context, registers it with Emscripten `GL` | Same; this is the only Emscripten-specific JS file |
| `avalonia/rendering/softwareRenderTarget.ts` | `putImageData` from WASM memory | Same; memory view comes from the WASM module instead of the .NET runtime |
| `avalonia/screens.ts` | Screens and window-management permission | `ferroui/screens.ts`, 1:1 |
| `avalonia/nativeControlHost.ts` | DOM element attachments | `ferroui/nativeControlHost.ts`, 1:1 |
| `avalonia/navigationHelper.ts` | Back handler, `window.open` | `ferroui/navigationHelper.ts`, 1:1 |
| `avalonia/caniuse.ts` | Mobile/TV detection, native file picker detection | `ferroui/caniuse.ts`, 1:1 |
| `avalonia/generalHelpers.ts` | Reflective property and method access on JS objects | Drop; `js-sys::Reflect` inside `interop/` covers the few uses |
| `avalonia/stream.ts` | Writable stream and blob helpers over runtime memory | `ferroui/stream.ts`, rewritten for a plain WASM memory view |
| `storage.ts`, `storage/storageProvider.ts`, `storage/storageItem.ts`, `storage/indexedDb.ts` | File pickers, storage items, bookmarks | `webapp/modules/storage.ts` and `storage/*`, 1:1, lazily imported |
| `avalonia-sw.ts` | Service worker for the save-picker polyfill | `ferroui-sw.ts`, 1:1 |
| `types/dotnet.d.ts` | .NET runtime typings | Drop |

## 6. Event loop and threading (question 4)

**Decision: single-threaded, browser-driven. No WASM threads.**

| Contract | Browser mapping | Notes |
|---|---|---|
| `IDispatcherImpl.Signal` | Post one task: `scheduler.postTask(cb, {priority: "user-blocking"})`, else `MessageChannel`, else `setTimeout(0)`; coalesced with a flag | Not a microtask: the browser must be able to paint and deliver input between jobs |
| `IDispatcherImplWithExplicitBackgroundProcessing` | Same mechanism at "user-visible" | Upstream deliberately does not use `isInputPending`; it starved low-priority jobs |
| `IDispatcherImpl.UpdateTimer` | One `setTimeout`, cleared and re-armed | All dispatcher timers multiplex onto it |
| `IDispatcherImpl.Now` | `performance.now()` | Never `std::time::Instant` on shared paths |
| `IRenderTimer` | `requestAnimationFrame` loop, `RunsInBackground = false`, tick carries the frame timestamp | Compositor commit and render run on the UI thread in the tick |
| Run loop | None. `IControlledDispatcherImpl.RunLoop` is not implemented | Nested frames, modal `ShowDialog`-style waits and any synchronous wait are impossible |
| Asynchronous platform calls (clipboard, pickers, font fetch, screen details, WebGPU device) | Promise on the JS side; completion delivered through an export, then posted through the dispatcher | `wasm-bindgen-futures::spawn_local` may be used inside the browser crate only |

What the core must provide:

- `AppBuilder::setup_with_lifetime(lifetime)` that returns, separate from any `start_with_..._lifetime` that blocks. The browser entry point is `start_browser_app(builder, "host-div-id", options)`; it sets up, creates the view, and returns to the browser.
- Application, lifetime and top-levels must be owned by thread-local or leaked state, never by `main`'s stack frame.
- A compositor mode in which the render side runs on the UI thread, driven by the render timer. If desktop adopts a render thread, this mode must remain.
- Platform services that are asynchronous upstream (`Task`-returning) must be completion-based in FerroUI: a callback, or a small promise type resolved through the dispatcher. No `block_on`, on any platform, in shared code.

Threads: upstream's multithreaded mode moves the UI to a worker with a blocking dispatcher loop and renders on another worker through `OffscreenCanvas`. It requires `SharedArrayBuffer`, hence cross-origin isolation (`Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` or `credentialless`), which static hosts often cannot set and which breaks third-party iframes and uncredentialed cross-origin resources. On the Rust side it needs a rebuilt standard library with atomics. FerroUI's `Rc`-based, UI-thread-affine object model gains nothing from it. Do not port `RenderWorker`, the worker branch of `WebRenderTargetRegistry`, or the event-grouper queue.

## 7. Windowing model (question 5)

- **Lifetime**: `BrowserSingleViewLifetime` implementing the single-view and single-top-level lifetime contracts. `main_view` set and get forward to the view.
- **Top-level**: `FerroView` (upstream `AvaloniaView`) owns an embeddable control root over `BrowserTopLevelImpl`. One per host element; constructing more views on other elements is allowed after `setup_browser_app`. A process-wide id-to-weak-top-level map routes JS callbacks.
- **No windows**: the windowing platform's `create_window` panics, as the upstream method throws. Anything in `FerroUI.Controls` that assumes a `Window` (dialogs, message boxes, window-hosted popups, menus that open native windows) needs an overlay-based path.
- **Popups**: `create_popup()` returns `None`; the overlay popup host and overlay layer in the top-level are used, positioned by the managed popup positioner. Popups are clipped to the canvas. The overlay popup host and the managed positioner must therefore be part of the first usable `FerroUI.Controls`, not a later addition.
- **Size and DPI**: `ResizeObserver` reports device pixels and `devicePixelRatio`; client size is `pixels / dpr`; `render_scaling` and `desktop_scaling` are both the device pixel ratio. The canvas backing size is set at the start of each draw from the last observed size. Zoom and monitor changes arrive as a scaling change.
- **Coordinates**: `point_to_screen` and `point_to_client` are identity upstream. Keep it for the MVP; correct it with `getBoundingClientRect` when screens are ported.
- **Focus**: the host `<div>` is focusable. When the top-level gets focus inside the framework, focus the host; while a text input client is active, focus the hidden input instead and return focus to the host when it is cleared. `lost_focus` is not raised upstream; wire `blur` on the host to it.
- **Activation**: `visibilitychange` maps to activated/deactivated (background kind).
- **Splash**: on the first rendered frame, add the `splash-close` class to the element with class `ferroui-splash` inside the host.

## 8. Input (question 6)

| Feature | Approach (as upstream unless noted) | Phase |
|---|---|---|
| Mouse, touch, pen | Pointer events on the host: `pointerdown/move/up/cancel`. `pointerType` selects the device; one mouse device per `pointerId`. Pressure, tilt, twist forwarded. Button from `event.button`, modifiers from `event.buttons` and key flags. Coalesced points fetched lazily in one call as a flat number array | 1 (mouse), 2 (touch, pen) |
| Pointer capture | `setPointerCapture`/`releasePointerCapture` on the host from the pointer's platform-capture hook | 2 |
| Wheel | `wheel` event, delta scaled to lines. Upstream divides by 50 and ignores `deltaMode`; honour `deltaMode` | 2 |
| Keyboard | `keydown`/`keyup` on the host; DOM `code` maps to physical key, DOM `key` to key and key symbol (port the `KeyInterop` tables). Unhandled `keydown` with a one-character `key` becomes a text-input event | 2 |
| Text input and IME | Hidden `<input>` positioned at the caret. `set_client` shows and focuses it and mirrors surrounding text and selection; `compositionstart/update/end` drive pre-edit text and commit; `beforeinput` supplies deletion ranges; `set_cursor_rect` moves the element using the caret helper | 2 |
| On-screen keyboard | `navigator.virtualKeyboard` `geometrychange` feeds the input pane | 3 |
| Clipboard | Async Clipboard API (`read`/`write` with `ClipboardItem`, `readText` fallback, paste-event fallback for old browsers). Formats: `text/plain`, `text/html`, `image/png`, custom as `application/<prefix>.<name>`. Needs the completion-based clipboard contract | 2 (text), 3 (rest) |
| Drag and drop | Incoming only: `dragenter/over/leave/drop` on the host, items wrapped as a data transfer, files as storage items. Drag-out is not implemented upstream | 3 |
| Cursors | Cursor factory producing CSS cursor strings; custom cursors as `url(data:...) x y, fallback`. Set on the host style | 2 |
| Context menu, browser gestures | `oncontextmenu` returns false; `touch-action: none` | 1 |

## 9. Fonts and text (question 7)

There are no system fonts. Upstream uses the Skia font manager as it exists in the WASM build and returns `SKTypeface.Default.FamilyName` as the default family; applications add fonts as embedded resources through font collections. What the SkiaSharp WASM binary embeds as its default typeface is **[U]**; with `skia-safe` on Emscripten assume **no usable default font**.

Decisions:

- Ship an embedded default UI font in a separate crate mirroring upstream's `Avalonia.Fonts.Inter` (`src/FerroUI.Fonts.Inter`, bytes via `include_bytes!`). The browser start-up registers it and sets it as the default family unless the application supplies font-manager options.
- The browser font manager is **collection-based**: installed families are the registered ones; typefaces are created from bytes; character fallback walks the registered collections in a configured order. On Skia this is a custom font manager fed from data; on the Vello configuration it is `skrifa`/`fontique` over the same byte store.
- Run-time sources, in this order of priority: embedded bytes (Phase 1); `fetch` of a URL to bytes, then register (Phase 3); Local Font Access API (`queryLocalFonts`, Chromium desktop only, permission prompt, **[V]**) as an optional opt-in (Phase 4 or never).
- Implications for the font-manager contract: fonts can be added after start-up, so the font manager needs a change notification that invalidates text layout and measure; no file paths anywhere in the contract (bytes or streams only); the default family name must be configurable rather than queried from the OS.
- Shaping: `FerroUI.HarfBuzz` unchanged on Emscripten. For `wasm32-unknown-unknown`, add a second text-shaper implementation on `harfrust` behind the same shaper contract. Since `harfrust` tracks HarfBuzz closely it could later replace the C++ build on all platforms; that is a separate decision.
- Fallback coverage (CJK, emoji) is the application's problem at first: document that fonts must be registered. Automatic fallback download is out of scope.

## 10. Storage, native control host, accessibility and the rest (question 8)

| Feature | Upstream | FerroUI |
|---|---|---|
| Storage provider | File System Access API where present, otherwise the `native-file-system-adapter` polyfill; bookmarks in IndexedDB; streams over `Blob.slice` and `FileSystemWritableFileStream`; a service worker makes save work in browsers without a native picker (marked unstable upstream) | Later (Phase 3; service worker Phase 4). Needs the completion-based storage contract and no `std::fs` in the storage abstractions |
| Launcher | `window.open(uri, "_blank")`; files unsupported | Phase 3, trivial |
| Native control host | Absolutely positioned DOM elements in a sibling `<div>` over the canvas; platform handle wraps a JS object | Phase 4. The handle type in the core must allow a non-integer handle |
| Screens | `window.screen`; multi-screen through `getScreenDetails` after permission | Phase 3; a single-screen stub in Phase 1 |
| Insets | Safe-area CSS variables, fullscreen as "system bar hidden" | Phase 3 |
| Platform settings | Theme, contrast, language, with change events | Phase 2 (theme), Phase 3 (rest) |
| System navigation | Back button through the history API | Phase 3 |
| Accessibility | None. The canvas is opaque to assistive technology | Later, and not a port: it needs a design (an ARIA mirror tree built from automation peers). Keep the automation-peer layer platform-neutral so this stays possible |
| Tray icon, window icon, z-order | Unsupported or stubbed | Stubs in Phase 1 |

## 11. Layout, tooling and phases (question 9)

```
src/Browser/FerroUI.Browser/
  Cargo.toml                 crate ferroui-browser
  lib.rs
  ferro_view.rs, browser_*.rs, cursor.rs, key_interop.rs, windowing_platform.rs, ...
  interop/                   one file per upstream Interop/*.cs
  rendering/                 one file per upstream Rendering/*.cs (no render_worker)
  storage/
  webapp/                    package.json, build.js (esbuild), tsconfig.json
    modules/ferroui.ts, storage.ts, ferroui-sw.ts
    modules/ferroui/..., modules/storage/...
  dist/                      built JS (generated, not committed)
src/FerroUI.Fonts.Inter/     embedded default font
samples/ControlCatalog.Browser/
  Cargo.toml, program.rs     binary crate (required by wasm-bindgen on Emscripten)
  wwwroot/index.html, app.css, main.js, favicon.svg
```

Workspace changes: the crates above as members, plus `src/FerroUI.OpenGL` (the OpenGL contracts, upstream `Avalonia.OpenGL`, which the Ganesh path of `FerroUI.Skia` and the WebGL render target implement); `skia-safe` features per target in the manifest of `FerroUI.Skia` (`graphite`+`metal` on Apple targets, `gl` on Emscripten); `FerroUI.Native` and `FerroUI.Desktop` are never built for the WASM target (the browser build selects its package).

Tooling:

- **Emscripten configuration**: plain `cargo build --target wasm32-unknown-emscripten` plus a repository script (`scripts/build-browser.sh` or an `xtask`) that runs esbuild, builds, and assembles `wwwroot` + `.js` + `.wasm` into a `dist` directory served by any static server. No trunk and no wasm-pack (wasm-pack has an open request for this target; trunk support **[U]**).
- Link settings (in `.cargo/config.toml` for the target) **[M]**: `linker = "em++"` (Skia needs the C++ runtime), `-sWASM_BINDGEN`, `-sMODULARIZE`, `-sEXPORT_ES6`, `-sENVIRONMENT=web` (the script of the module leaves out Node.js and worker support), `-sMAX_WEBGL_VERSION=2`, `-sALLOW_MEMORY_GROWTH=1`, `-sEXPORTED_RUNTIME_METHODS=GL,HEAPU8` (the JS side must reach Emscripten's `GL` object and the module memory), `-sINVOKE_RUN=0` (the host page starts the application; without it `main` also runs twice), `-sSTACK_SIZE=8MB` (the default stack of 64 KB is too small for layout and markup loading), `-sGL_ENABLE_GET_PROC_ADDRESS=1` (the OpenGL entry points of a context are resolved by name), `--js-library=src/Browser/FerroUI.Browser/emscripten/wasm_table_mirror.js` (keeps the cache of function table lookups in size-optimised links, see section 18), and `EMCC_CFLAGS="-s ERROR_ON_UNDEFINED_SYMBOLS=0"` as rust-skia requires. No `-Cpanic=abort` and no `-Crelocation-model=static`. The backend imports its script module by a relative specifier, so there are no `wasm-bindgen` snippets to copy: `scripts/build-browser.sh <application>` assembles the site from the host page, `ferroui.js` and the `.js`/`.wasm` pair of the application: an example of the browser crate (`target/wasm32-unknown-emscripten/<profile>/examples/`) or a binary package of the workspace with its host page in the `wwwroot/` of the package (`target/wasm32-unknown-emscripten/<profile>/`, where the module of a binary is named after the target with `-` replaced by `_`), built with the `browser` profile of the workspace (section 18), plus the files that build scripts of the application leave in `$OUT_DIR/browser-site/` (asset bundles).
- **Browser scripts** (`scripts/browser/`): `setup.sh` installs the pinned toolchain (locally and, through the composite action `.github/actions/browser-toolchain`, in CI); `harness.mjs` serves a site and drives headless Chrome over the DevTools protocol (real pointer, wheel and key events, resize, screenshots, the console and the error log); the tests under `tests/` use it: `themed_view.test.mjs` (the input matrix and the services of the backend), `storage_view.test.mjs` (the storage provider) and `control_catalog.test.mjs` (the ControlCatalog site: start-up, WebGL2 and 2D-canvas rendering, resize, console errors, no upstream brand asset, and the drawer, navigation, a button click and typing into a text box driven with real events; run by the `browser` job of CI and before every Pages deployment); `capture.mjs` takes a screenshot of a site; `first-frame.mjs` measures the time from navigation to the first frame of a site (module downloaded, first draw, frame on screen, splash screen closed) over several loads with a fresh profile each, can write a CPU profile, emulate a network (`--throttle <Mbit/s>,<ms>`) and report the phases of a load (`--phases`: every file downloaded, module compiled and instantiated, the performance marks of the page; section 19); `wasm-size-report.py` attributes the bytes of a WebAssembly module to sections, embedded files, crates and generic families (section 18); `module-sizes.mjs` prints the raw, gzip and brotli sizes of the files of a site (and, with `--github-output`, the sizes of its WebAssembly module as step outputs); `render-placeholder-assets.mjs` draws the placeholder artwork of the published catalog.
- **Publication**: `.github/workflows/pages.yml` builds the ControlCatalog site, tests it and deploys it to GitHub Pages on pushes to `main` and on manual runs. It deploys only while the WebAssembly module stays within the gzip budget `PUBLISHED_MODULE_GZIP_BUDGET_MB` of the workflow (14 MB: raised from 12 MB when the themes moved to compiled markup, stage E4 of the XAML compiler, which brought the catalog module to 11.42 MB with gzip; the budget is that size with about 15 % headroom, rounded up); above it the size budget step of the deploy job fails before the deployment, and the raw and gzip sizes are in the summaries of the run. Only `main` is deployed (a manual run from another branch builds and tests only), and only the deploy job has `pages: write` and `id-token: write`. Every action of the workflows is pinned to a commit. The toolchain steps are shared with CI through the composite action `.github/actions/browser-toolchain`. GitHub Pages compresses responses itself (gzip, including `application/wasm`; no brotli was offered for `Accept-Encoding: br, gzip` when measured on 2026-10-05) and does not serve precompressed files, so the site carries none.
- **`wasm32-unknown-unknown` configuration**: trunk, with the same `webapp` bundle.
- TypeScript: keep upstream's esbuild script and ESLint configuration; three bundles as upstream. Asset fingerprinting and import maps are not needed initially.

Phases:

| Phase | Content | Exit criterion |
|---|---|---|
| 0 | Core constraints of section 12 applied in ongoing core and desktop work | No new violations; CI job `cargo check --target wasm32-unknown-emscripten` for `FerroUI.Base` and `FerroUI.Controls` |
| 1a | Toolchain spike: `skia-safe` (`gl`) + `harfbuzz-sys` + `wasm-bindgen` linked under emsdk 6.0.10+, drawing one rectangle and one shaped text run with Skia raster into a 2D canvas | Done 2026-10-05 in Chrome (WebGL2 and raster); size and start-up measured; binding approach confirmed. Firefox and Safari are still to be checked |
| 1b | **MVP**: `start_browser_app`, single-view lifetime, `FerroView`, `BrowserTopLevelImpl`, dispatcher, `requestAnimationFrame` render timer, canvas surface with resize and DPR, software and WebGL2 render targets, embedded default font, mouse pointer input, stub services | **A window-equivalent top-level in a `<div>` shows a basic control (a button with text inside a border), laid out and rendered through the normal compositor path, re-rendering correctly on resize and zoom, and reacting to hover and click** |
| 2 | Keyboard, wheel, touch, pen, pointer capture, cursors, text input and IME, text clipboard, theme setting, overlay popups verified (tooltip, flyout, combo box) | A text box is usable including IME; popups work |
| 3 | Rich clipboard, drag and drop, storage provider, launcher, screens, insets, input pane, navigation, activation, font fetch, multiple views, ControlCatalog sample | ControlCatalog runs |
| 4 | Vello backend and `harfrust` shaper on `wasm32-unknown-unknown`; native control host; service worker; accessibility design | Second configuration renders the catalog |

## 12. What to keep in mind now in core and desktop work (question 11)

1. **No blocking loop in the core.** The run loop belongs to the lifetime and platform, behind a separate, optional controlled-dispatcher contract. Setup must be callable without running.
2. **No blocking waits.** No nested dispatcher frames, no synchronous invoke-and-wait, no condition variables or channels with blocking receive, no `block_on`, in `FerroUI.Base` or `FerroUI.Controls`. Modal behaviour must be expressible with completions.
3. **Asynchronous platform services are completion-based** (clipboard, storage, launcher, screen details, font loading, graphics device creation). Decide the one completion type now, without an async-runtime dependency.
4. **Time comes from the platform.** Use the dispatcher's `now` and the render-timer tick timestamp. `std::time::Instant::now()` and `SystemTime::now()` panic on `wasm32-unknown-unknown`; keep them out of shared code even though Emscripten supports them (**[M]**: both work there, `std::fs` works against an in-memory file system, and `std::thread` spawning fails with `Unsupported`).
5. **No threads assumed.** No `std::thread::spawn`, no background render thread requirement, no timers implemented with sleeping threads. The compositor must run with commit and render on one thread. `UiStatic` and `thread_local!` are fine on both targets.
6. **`Rc` is right; do not add `Send`/`Sync` bounds to platform contracts.** JS handles (`JsValue`) are neither. Contracts that upstream documents as callable from any thread (the render-timer tick setter) should not force `Send` in the trait.
7. **No `std::fs`, `std::env`, `std::process`, `std::net` in shared paths.** Assets and fonts are bytes behind the asset-loader contract; file-path overloads belong to desktop-only code. Logging sinks must be pluggable (browser console).
8. **Popups without windows.** Overlay layer, overlay popup host and managed popup positioner are first-class; controls must not require a `Window`.
9. **Top-level without window.** `TopLevel` over `ITopLevelImpl` and the embeddable control root must be complete on their own; platform features are looked up with a `try_get_feature`-style query that may return nothing.
10. **Platform handles are not always integers.** The handle abstraction must be able to carry an opaque platform object.
11. **Renderer abstraction stays Ganesh-capable** and keeps a framebuffer path and a not-ready state (section 4). Do not let Graphite-only assumptions leak above `FerroUI.Skia`.
12. **Font manager is dynamic and path-free** (section 9).
13. **No reliance on unwinding for control flow.** The Emscripten configuration builds with `unwind` (the only strategy the pinned toolchain supports there, section 3) and `catch_unwind` works, but `wasm32-unknown-unknown` configurations are commonly built with `abort`: no `catch_unwind` for control flow; destructors must not be the only place where essential state is restored after a panic.
14. **32-bit `usize`.** Collection indices, hashes and pointer-sized casts must not assume 64 bits; keep 64-bit timestamps and ids explicitly `u64`/`i64`.
15. **Dependency hygiene.** Crates used by `FerroUI.Base` and `FerroUI.Controls` must build on both WASM targets; platform-specific crates are added under `cfg(target_...)`. Anything using `getrandom` needs its browser backend selected in the application crate.
16. **Size discipline.** Avoid large generic instantiations and `std::fmt`-heavy paths in hot generic code; WASM size is a product feature.

## 13. Verification status

### Verified by the feasibility spike (2026-10-05)

Measured on macOS arm64 with Emscripten 6.0.10, Rust 1.90.0, `skia-safe` 0.153.3, `harfbuzz-sys` 0.8.0, `wasm-bindgen` 0.2.129. Browser evidence is from **headless Google Chrome 154 only** (ANGLE on Metal).

- `skia-safe` with the `gl` feature builds for `wasm32-unknown-emscripten` from rust-skia's prebuilt binary (first build 1.5 minutes including the download); feature set as listed in section 3.
- `harfbuzz-sys` `bundled` builds under `em++` and shapes text.
- `wasm-bindgen` works on the target with the corrections of section 3: panic strategy `unwind`, `-sINVOKE_RUN=0`, matching command-line tool on `PATH`, linker `em++`.
- `skia-safe`, `harfbuzz-sys` and `wasm-bindgen` link into one module with the settings of section 11. A page that creates a WebGL2 context, registers it with the module's `GL` object and has Skia (Ganesh) draw shapes and a HarfBuzz-shaped glyph run renders correctly; so does the raster path blitted with `putImageData` from a view over the module memory. The module is 4.6 MB (1.9 MB with gzip) and instantiates in about 36 ms from a local server.
- A typeface can be created from bytes with Skia's font manager on the target; there is no default typeface, as section 9 assumes.
- The core crates (`FerroUI.Base`, `FerroUI.Controls`, the markup crates and the run-time loader, both themes, `FerroUI.HarfBuzz`, the ControlCatalog library) compile for the target unchanged. `FerroUI.Skia` needs its Skia features per target.
- No published Skia binary combines Graphite and Ganesh; with unsupported feature combinations the build of `skia-bindings` silently falls back to compiling Skia from source (and currently fails there). Feature resolution must be checked with `cargo tree -e features` before building for a new target.

### Verified with the backend skeleton (2026-10-05)

- Rust imports the script module by a plain relative specifier (`#[wasm_bindgen(raw_module = "./ferroui.js")]`), so the crate checks and tests without the bundle; the site places `ferroui.js` next to the script of the WebAssembly module. Exports of a library crate reach the page; properties of script objects are read with typed getters, without `js-sys`.
- The application does not start when the module is instantiated: the host page creates the module, hands it to the script side (`FerroExports.attach(runtime)`) and then calls the exported entry point. Every export is therefore resolved before the first callback fires.
- The `themed_view` example of `FerroUI.Browser` (Fluent theme, embedded Inter font, run-time XAML loader) renders correctly in headless Chrome 154 through WebGL2 (Skia Ganesh), through the 2D-canvas path (`?RenderingMode=Software2D`), with the dark variant at a device scale factor of 2, and re-lays out when its host element is resized. The page is up within a few seconds from a local server.
- Additional link settings: `-sSTACK_SIZE=8MB` (the default 64 KB stack is too small) and `-sGL_ENABLE_GET_PROC_ADDRESS=1` (the OpenGL entry points of the context are resolved by name).
- Size of that example, release profile of the workspace, not optimised for size: 48.7 MB of WASM, 10.2 MB with gzip. Section 18 reduces it to 25.9 MB, 8.3 MB with gzip (`main` of 2026-10-06).

### Verified with the ControlCatalog host (2026-10-05)

Measured on Linux x86_64 with Emscripten 6.0.10, Rust 1.90.0 and `wasm-bindgen` 0.2.129; browser evidence from headless Chromium 141 (the Playwright build of the cloud image) with WebGL through SwiftShader.

- `samples/ControlCatalog.Browser` (crate `control-catalog-browser`) starts the catalog with the Fluent theme in the single-view lifetime; `scripts/browser/tests/control_catalog.test.mjs` passes for `?RenderingMode=WebGL2` (the canvas has a WebGL2 context) and `?RenderingMode=Software2D` (a 2D context): the application is up 8 to 9 s after navigation from a local server, the main view is drawn, it is laid out again when the page grows from 1024x700 to 1440x900 and shrinks to 800x600, and nothing is logged as an error (`images/control_catalog_browser.png`). SwiftShader satisfies `failIfMajorPerformanceCaveat` when Chrome runs with `--use-angle=swiftshader --enable-unsafe-swiftshader`, which the harness passes.
- Input in the catalog, with real pointer and key events (`control_catalog.test.mjs`): the toggle button opens the drawer of the main view at 600 pixels (overlay mode); the drawer reaches the pages Basic Input, Buttons, CheckBox, Text and TextBox, each showing its content; a click on the first button of the Buttons page raises its `Click`; text typed into the text box of the TextBox "First Look" sample, and Backspace, change its text and the caption bound to it. The host exports `catalogState()` (JSON: the drawer, the current page, whether the navigation page is navigating, the focus, and the visible text blocks, text boxes and buttons with their bounds and whether the pointer reaches them), and the host page exposes the module as `globalThis.controlCatalog`. The navigation page ignores a navigation while it runs one (as upstream), so the test waits for a navigation to end before the next click.
- The catalog logs the binding warnings of the gap the desktop host shows too (`$parent[MainView].ViewModel`, plain properties of a class through a base handle; see `CRITICAL-PATH.md`, item 23a); they go to the console log, as the console trace listener of the managed original writes them.
- Size, release profile of the workspace, not optimised for size: 84.8 MB of WASM, 32.2 MB with gzip at level 9, 26.4 MB with brotli at quality 11; the script of the module 195 kB (39 kB gzip). A Pages deployment needs the module within 12 MB with gzip; section 18 reduces it to 31.9 MB, 9.5 MB with gzip.

### Still not verified

- Firefox, Safari and mobile browsers; WebGL1; the `failIfMajorPerformanceCaveat` fallback to the raster path.
- Whether all of `web-sys`, `js-sys` and `wasm-bindgen-futures` are usable on the Emscripten target (the boundary rules of section 5 keep them out of the port).
- Whether a newer Rust toolchain or a rebuilt standard library allows `-Cpanic=abort` on the target, and what it would save.
- Run-time speed (layout, rendering, input) of the size-optimised module after start-up; start-up and size are measured in section 18.
- `vello_gpu` and `vello_cpu` behaviour on Emscripten; the `vello_hybrid` to `vello_gpu` rename date and versions (secondary source; only the existence and description of `vello_gpu` were confirmed in the README).
- Vello feature gaps against the drawing-context contract and its current text-rendering quality.
- WebGPU availability per browser and OS (secondary sources, not vendor documentation).
- What default typeface, if any, the SkiaSharp WASM binary embeds, and how upstream apps get text without registering a font.
- trunk support for `wasm32-unknown-emscripten`.
- Rust standard-library requirements for WASM threads on either target (not investigated further because threads are rejected).

## 14. Upstream quirks: do not copy as is

| Location | Observation | FerroUI action |
|---|---|---|
| `timer.ts` / `TimerHelper.cs` | The animation-frame callback calls the export without the timestamp although the managed side takes a `double` | Pass the timestamp |
| `input.ts` key handlers | Handlers return a promise and call `preventDefault` in `.then`; `keydown` prevents default unless the event was handled while a clipboard read is pending, `keyup` prevents default when **not** handled. In effect almost every key's default action is suppressed while the host has focus | Fixed: the exports answer synchronously and `shouldPreventDefault` decides while the event is dispatched. Keys of a composition and dead keys are the browser's; a handled key is prevented; an unhandled key keeps its default action when it is a Ctrl/Meta/Alt combination, a function key or Tab, and is prevented otherwise (characters, Space, arrows, paging keys, Backspace). `scripts/browser/tests/themed_view.test.mjs` holds the test matrix |
| `input.ts` pointer unsubscription | Removes `pointerover` instead of `pointermove`; the `beforeinput` listener is never removed | Fixed; the subscription is ended when the top-level is disposed |
| `BrowserInputHandler.OnWheel` | Fixed divisor 50, `deltaMode` ignored | Fixed: `deltaMode` is passed. Pixels are divided by 50 (the scroll presenter scrolls 50 units per delta of 1); lines by 3 (a delta of 1 is one wheel notch in the Windows and X11 backends, and a notch is three lines); a page is the size of the view divided by 50 |
| `softwareRenderTarget.ts` | A premultiplied framebuffer is passed to `putImageData`, which expects straight alpha; translucent pixels over page content come out wrong | Fixed: the frame is converted to straight alpha into a retained buffer before it is put on the canvas (opaque and transparent pixels are copied unchanged); `webapp/tests/software-blit.test.mjs` checks the composited pixels in headless Chrome |
| `BrowserTopLevelImpl` | `PointToScreen` is the identity; `LostFocus` is never raised; `Dispose` does not unsubscribe input | `LostFocus` is raised and input is unsubscribed (below). `point_to_screen` stays the identity: a page knows where its window is on the screen (`screenX`) but not where the viewport is inside the window (the browser's own bars), so a corrected value would still be wrong by an unknown offset |
| `input.ts` `getModifiers` | The barrel button of a pen is detected by comparing the **event type** (`"pointerdown"`, ...) with `"pen"`, so it is never reported and the button counts as the right mouse button | Fixed: the pointer type decides; tested with synthetic pen and mouse events |
| Focus leaving the view (not in the original) | The framework is never told that the page moved the keyboard focus away | The host reports `focusout` when another element of the same document takes the focus, or nothing does while the document still has the focus (a click on the page). A plain `focusout`/`blur` is not used: it also fires when the window or the tab is deactivated, and the browser restores the focused element afterwards without telling anyone, so the view keeps its focused control then. While the loss is reported the text input method does not give the focus back to the host, and the hidden input element is not a tab stop (`tabIndex = -1`), so Tab leaves the view in one step |
| `BrowserInputHandler.OnKeyDown` | Every unhandled key whose `key` is one character is raised as text, whatever the modifiers: Ctrl+R types "r" (and counts as handled) | Fixed: with Meta, or Control without Alt, the key is a command and no text; Shift, Alt and AltGr (Control with Alt) still produce characters |
| `BrowserTextInputMethod` focus calls | `SetCursorRect` and showing the input element focus the hidden input element unconditionally | Not while the page reports that the focus left the view: a text box that moves its cursor as it loses the focus would take the focus back from the element the user went to |
| Key events without `code` or `key` | The exports take non-null strings | `code` and `key` are optional across the boundary (some virtual keyboards and autofill send such events): an unknown key, no text |
| `BrowserInputHandler.OnPointerMove` | The coalesced-points loop steps the index by the item size while bounding by point count | Fixed: every coalesced point but the last (the event itself) becomes an intermediate point |
| Promise-based imports (`Task` returned by `[JSImport]`) | The runtime marshals a promise into a task | One completion mechanism: an asynchronous import returns its promise, `interop/completion_helper.rs` gives it a request id and hands both to `CompletionHelper.track`, and the page answers through `CompletionHelper_OnResolved`/`CompletionHelper_OnRejected`; the future of the request is woken and polled by the dispatcher. Dropping the future forgets the request. Tested natively behind a private tracker trait |
| `BrowserDataTransferHelper.GetReadableItemFormats` | `hasSupportedImage = formatString is "image/png"` is assigned in the loop, so the bitmap format is offered only when the PNG is the last format of the item | Fixed: accumulated |
| `input.ts` `tryGetReadableDataItemValue` | Reads a string item of a drag operation with `getAsString`, whose callback runs after the event: the value is always empty | Fixed: the items of a drag event carry its data transfer, and `getData` reads the value while the drop is dispatched (in the other drag events the browser answers `""`, as for every page) |
| `dom.ts` `getSafeAreaPadding` | Returns left, top, bottom, right; `BrowserInsetsManager` reads left, top, right, bottom, so bottom and right are swapped | Fixed: returns left, top, right, bottom |
| `input.ts` paste fallback | `keydown` keeps its default action while a clipboard read waits for the paste event; `keyup` rejects the read without a reason; the read resolves with the items of the paste event, whose values are read after the event, when `getAsString` no longer calls back, so the read never completes | Kept within the synchronous key policy: a handled `keydown` that started such a read is not prevented (the paste is its default action). Fixed: the paste listener reads the string values with `getData` while the event is dispatched (a `string` readable item with its format); files stay items of the event. The rejection carries an error message, reported as a clipboard error of kind `Other` |
| `navigationHelper.ts` `addBackHandler` | After a handled request it goes `history.forward()` to the entry it pushed; that navigation fires `popstate` too, which it takes for a second back request, so every back navigation is reported twice | Fixed: the `popstate` of its own forward navigation is skipped |
| `Interop/NavigationHelper.AddBackHandler` | Not called anywhere since upstream #15849: the back navigation of the browser never reaches `BrowserSystemNavigationManagerImpl` | Kept: the navigation manager is registered, and `interop::navigation_helper::add_back_handler` is public for an application that wants the back button (it pushes a history entry). The callback is the export `NavigationHelper_OnBackRequested` instead of a function passed to the page |
| `ClipboardImpl`, `BrowserDataTransferHelper.TryGetValue` | Tell string formats from byte formats by the data type of the requested format (`DataFormat<string>`, `DataFormat<byte[]>`) | The formats of the framework carry no data type: when writing, the type of the value decides; when reading, formats whose name on the page is a `text/*` media type are strings and the others bytes, the rule the formats of an item are created with. So a string application format (`create_string_application_format`, `application/frn-fmt.*` on the page) is read as bytes, where the original returns its text; the application gets the UTF-8 bytes of the text instead |
| `BrowserInputHandler.OnDragEvent` | Writes `DragDropEffects.ToString()` in lower case to `dropEffect`: a combination (`"copy, move"`) is not a value the page accepts | Kept; documented on `drop_effect_name` |
| `BrowserInsetsManager.IsSystemBarVisible` | The getter answers whether the page is in full screen, while the setter requests full screen when the bars are to be hidden: the getter is the inverse of what its name says | Kept (`is_system_bar_visible` returns `is_fullscreen`) |
| `ScreenHelper.checkPermissions` | `permissions.query({ name: "window-management" })` rejects in browsers that do not know the name (Firefox, Safari), and nothing catches it | Kept: the rejection is reported by the browser as unhandled; the screens work without the details |
| `stream.ts` `write` | The copy fallback writes the original span, not the copy | Fixed: `stream.ts` is rewritten over `FerroExports.runtime.HEAPU8`, fetched per call; a written chunk is copied out of the module memory before the call returns (the writable stream queues it). The storage tests write a 3 MB file while the memory grows |
| `BrowserStorageProvider.cs` | Script exceptions propagate to the caller of `OpenFileBookmarkAsync`, `OpenFolderBookmarkAsync` (`OpenBookmark`), `TryGetWellKnownFolderAsync` (`ImportStorage`), `GetBasicPropertiesAsync` (`GetProperties`), `SaveBookmarkAsync` (`SaveBookmark`), `ReleaseBookmarkAsync` (`DeleteBookmark`), and of `GetFileAsync`/`GetFolderAsync` for messages other than the three suppressed ones; a disposed item throws `ObjectDisposedException` | Contract seam: these contracts have no error channel. The `JSException` becomes `None` (bookmark open and save, well-known folder, `get_file_async`, `get_folder_async`), properties without values, or `()` (bookmark release); a disposed item has an empty name, and its fallible operations fail with an I/O error |
| `WriteableStream.cs` | `Seek` returns the new position without storing it; the `Position` setter seeks to the current position, ignoring the value; writes do not extend `Length`; the initial `Length` is the old size of the file although `openWrite` truncates it (`keepExistingData: false`), so `Seek(.., End)` lands past the content | Fixed: the position is stored, the setter seeks to the value, writes extend the length, and the length starts at 0 |
| `BlobReadableStream.cs`, `WriteableStream.cs` | `Read` and `Write` throw: only `ReadAsync` and `WriteAsync` work, and `WriteAsync` reports the failure of each write. The storage contracts of the port hand out synchronous `Read` and `Write`, and the browser cannot block | Contract seam. Reading: opening a file reads the blob into memory first, and fails with `UnexpectedEof` (upstream's `EndOfStreamException`) when the blob yields fewer bytes than its length; `read_async` keeps the sliced read. Writing: `write` issues the write without waiting; `write` and `flush` report a failure that has already settled, but cannot wait for one (`flush_async` and `write_async` do). Dropping the stream closes it. That close stays registered for its file (matched with `isSameEntry`, whichever item opened it): opening the same file or reading its properties waits for it, and a failed write or close is logged (area `BrowserPlatform`) and reported as an error by the next opening of that file. A close that never settles holds back only its own file. Remaining difference: code that writes through the synchronous stream and never opens the file again sees a failure only in the log |
| `storageItem.ts` `moveAsync` | Passes the storage item instead of its handle to `FileSystemHandle.move`, which rejects it; the move resolves to nothing, so `MoveAsync` returns `null` | Fixed: the destination handle is passed and the moved handle returned; the storage tests move a file |
| `storageItem.ts` `verifyPermissions`, `saveBookmark` | Skip permissions and bookmarks only when the browser has no native picker. With `PreferFileDialogPolyfill` the handles are the polyfill's even where native pickers exist: its save handle reports no write permission, so every save fails, and its handles cannot be stored in IndexedDB | Fixed: both also require a native `FileSystemHandle`; the storage tests save through the polyfill's download |
| `nativeControlHost.ts` `releaseChild` | Forgets the element of the attachment without removing it from the element of the view | Kept: the host detaches the control (`attachTo(null)` removes the element) before it disposes the attachment, so nothing is left behind in the flows of the native control host |
| `BrowserNativeControlHost.CreateNewAttachment` | Disposes the attachment when setting its host throws | The calls into the page do not fail with a Rust error, so there is nothing to dispose; a host of another platform, a handle that is not a `JsObjectControlHandle` and a disposed attachment panic, as the casts and `ObjectDisposedException` of the original throw. The core contract `INativeControlHostImpl` has `as_any` for the cast to the browser host |
| `NativeControlHost` (core, not the backend) | Moves the native control when the bounds or the visibility of an ancestor change, not when a render transform does. A page that a navigation page slides in (`PageSlide` animates a `TranslateTransform`) places its native controls at the start of the slide, a page width to the right, and they stay there until a bound changes (seen on the Native Embed page of the catalog) | Kept, as the core control is ported as is; `control_catalog.test.mjs` resizes the view after the navigation |
| `input.ts` pointer events over native controls | The pointer and wheel handlers of the view also take the events whose target is a native control (an element inside the native host): the coordinates (`offsetX`, `offsetY`) are then relative to that element, and a press is captured implicitly by the framework, so the release and the `click` go to the element of the view and the native control never gets its click (seen with the button of `embed.js`) | Fixed: an event whose target is inside a native control is left to it. While the view captures the pointer the target is the element of the view, so a drag started on the view continues over native controls. Keyboard and focus events are unchanged. A pointer that moves from the view onto a native control without capture gives the view no move or leave, so the pointer-over state of the control under it stays until the pointer comes back (upstream sent those moves, with coordinates relative to the native element) |
| `BrowserStorageProvider.cs` storage items of a data transfer | The import of the storage module started with a drag event is not awaited (`_ = AvaloniaModule.ImportStorage()`), and `JSStorageFile.OpenReadAsync` does not import it, so reading a dropped file right away can call into a module that is not loaded yet (`StorageItem` undefined) | Fixed: every operation of a storage item awaits the import (a no-op once the module is loaded) |
| `JSStorageItem.MoveAsync` / `DeleteAsync` after a write | A writable stream of the file that is still closing locks its handle, so a move or a delete right after a write fails (`NoModificationAllowedError`) | Fixed: a move or a delete first waits for the closes pending on the file, as an opening of the file does |
| `BrowserPlatformOptions.RegisterAvaloniaServiceWorker`, `AvaloniaServiceWorkerScope` | Options of the service worker, marked unstable | `register_ferro_service_worker` and `ferro_service_worker_scope`; the worker is `ferroui-sw.js`, which `scripts/build-browser.sh` places at the root of the site (its scope is its directory, and the polyfill finds it with `getRegistration()` against the address of the document). Registration is not awaited, as the `void` import of the original; a failure is an unhandled rejection of the page |
| `avalonia-sw.ts` `MessagePortSource` | Asks the page for the next chunk (`PULL`) only after it received one, and has no `pull`; the polyfill's writer waits for a first `PULL` before it writes anything, so a save through the worker never sends data and the download never starts (seen in headless Chromium: the frame of the download stays in navigation) | Fixed: `pull` asks for a chunk whenever the stream wants data, the first one included, and a received chunk is only enqueued (what the comment of the original describes). `storage_view.test.mjs` downloads a file through the worker |
| `ControlCatalog.Browser` host: the resources of the sample (host-level difference, not a change of the framework) | The runtime downloads every resource of the sample's assembly (the 79 photographs and 5 fonts) with the other boot resources before `Main` runs, and `AssetLoader` opens them synchronously | The asset API and its semantics are unchanged (synchronous `AssetLoader`, `register_asset_bundle`; a bundle registered later is seen after `invalidate_assembly_cache_all`, as for any newly registered assembly). The catalog's build script splits the assets by the pages that use them (`samples/ControlCatalog/build/page_bundles.rs`: a scan of the documents and Rust files of the sample for asset paths, font-collection directories, file patterns and run-time joined file names, and of the classes and modules each source reaches from `App.xaml`, the main view and window, the start page, and each entry of the page list). `control-catalog.assets` holds what the start-up reaches and what no source names (reported as build warnings); one bundle per page or per set of pages holds the rest, so that no page downloads what it does not use, and an asset of 1 MB or more (the CJK font) has a bundle of its own; assets that only sources nothing reaches name (the demos of `NavigationDemoPage`, whose class is not ported) are in `control-catalog.unreached.assets`. `control-catalog.assets.json` lists the bundles by page header. The host page registers the start-up bundle and starts the application; before the catalog creates a page, `MainWindowViewModel.navigate_to_async` awaits the hook `control_catalog::PageAssets` (not in upstream, set only by the browser host; the desktop host sets none and embeds every asset), which fetches and registers the bundles of the page (`page-assets.js`, `page_assets_browser.rs`); of navigations asked for meanwhile the last one wins, and a page whose bundle cannot be fetched is not created. After the first frame the other bundles are fetched one at a time while the page is idle (`?PrefetchAssets=false` turns that off). Fonts: `ferres://ControlCatalog/Assets/Fonts#Family` loads every font of the directory into one font collection, created when a page first uses it and kept by the font manager, so every page that names the directory (TextBlock, TextBox) waits for the whole directory, the CJK font included; nothing at start-up names it and the font fallbacks of the catalog are empty, so deferring it changes no other page. `tests/asset_bundles.rs` checks that every page of the list, and every sample of its entry, opens only assets of the start-up bundle and of its own bundles; `control_catalog.test.mjs` checks the downloads, the pictures and the CJK glyphs in the browser. Sizes and start-up times: section 19 |
| `EmbedSample.Browser.cs` | Imports `embed.js` on first use (`JSHost.ImportAsync`) and adds the button once the import completes | `embed.js` is imported statically with the module (the boundary has no run-time module import) and the button is added while the control is created |
| Threaded mode | Reaches into non-public runtime APIs | Not ported |

## 15. File-level inventory of `src/Browser/Avalonia.Browser` (question 10)

Phase key: 1 = MVP, 2 = input, 3 = services, 4 = late, – = not ported. Rust paths are relative to `src/Browser/FerroUI.Browser/`.

### C#

| Upstream file | Purpose | FerroUI counterpart | Phase |
|---|---|---|---|
| `Avalonia.Browser.csproj` | Project, native asset references, bun/esbuild targets | `Cargo.toml` + build script for `webapp` | 1 |
| `build/Avalonia.Browser.props` | `AvaloniaAllowWebGl2` default | Cargo feature / documented link flag | 1 |
| `build/Avalonia.Browser.targets` | emcc flags, exported runtime methods, static web assets with fingerprinting | `.cargo/config.toml` flags + `scripts/build-browser.sh`; no fingerprinting | 1 |
| `BrowserAppBuilder.cs` | Options, rendering-mode enum, `StartBrowserAppAsync`, `SetupBrowserAppAsync`, `UseBrowser` | `browser_app_builder.rs` (`BrowserPlatformOptions`, `BrowserRenderingMode`, `start_browser_app`, `setup_browser_app`, `use_browser`) | 1 |
| `BrowserSingleViewLifetime.cs` | Single-view lifetime | `browser_single_view_lifetime.rs` | 1 |
| `AvaloniaView.cs` | Host element to embeddable root, splash removal | `ferro_view.rs` | 1 |
| `WindowingPlatform.cs` | Service registration, thread detection, windowing platform that refuses windows | `windowing_platform.rs` (no thread detection) | 1 |
| `BrowserRuntimePlatform.cs` | Runtime info (mobile/desktop/TV), asset loader registration | `browser_runtime_platform.rs` | 1 |
| `BrowserTopLevelImpl.cs` | Top-level implementation, feature lookup, id map | `browser_top_level_impl.rs` | 1 |
| `BrowserSingleThreadedDispatcherImpl.cs` | Dispatcher implementation | `browser_single_threaded_dispatcher_impl.rs` | 1 |
| `BrowserInputHandler.cs` | Raw input event construction and dispatch | `browser_input_handler.rs` (no event grouper) | 1–2 |
| `BrowserMouseDevice.cs` | Mouse device with DOM pointer capture | `browser_mouse_device.rs` | 2 |
| `KeyInterop.cs` | DOM `code`/`key` to key tables | `key_interop.rs` | 2 |
| `Cursor.cs` | CSS cursor implementation and factory | `cursor.rs` | 2 |
| `BrowserTextInputMethod.cs` | Text input method over the hidden input | `browser_text_input_method.rs` | 2 |
| `BrowserInputPane.cs` | Virtual keyboard occlusion | `browser_input_pane.rs` | 3 |
| `BrowserPlatformSettings.cs` | Theme, contrast, language | `browser_platform_settings.rs` | 2–3 |
| `ClipboardImpl.cs` | Clipboard read/write | `clipboard_impl.rs` | 2–3 |
| `BrowserClipboardDataTransfer.cs` | Clipboard data transfer wrapper | `browser_clipboard_data_transfer.rs` | 3 |
| `BrowserClipboardDataTransferItem.cs` | Clipboard item wrapper | `browser_clipboard_data_transfer_item.rs` | 3 |
| `BrowserDragDataTransfer.cs` | Drag data transfer wrapper | `browser_drag_data_transfer.rs` | 3 |
| `BrowserDragDataTransferItem.cs` | Drag item wrapper | `browser_drag_data_transfer_item.rs` | 3 |
| `BrowserDataFormatHelper.cs` | Data format to MIME mapping | `browser_data_format_helper.rs` | 2 |
| `BrowserDataTransferHelper.cs` | Reading values from JS data items | `browser_data_transfer_helper.rs` | 3 |
| `BrowserScreens.cs` | Screens | `browser_screens.rs` (single-screen stub in 1) | 3 |
| `BrowserInsetsManager.cs` | Safe area, fullscreen | `browser_insets_manager.rs` | 3 |
| `BrowserSystemNavigationManager.cs` | Back request | `browser_system_navigation_manager.rs` | 3 |
| `BrowserActivatableLifetime.cs` | Visibility to activation | `browser_activatable_lifetime.rs` | 3 |
| `BrowserNativeControlHost.cs` | DOM native control host | `browser_native_control_host.rs` | 4 |
| `JSObjectControlHandle.cs` | Platform handle wrapping a JS object | `js_object_control_handle.rs` | 1 |
| `WinStubs.cs` | Icon loader stub | `win_stubs.rs` | 1 |
| `Interop/AvaloniaModule.cs` | Module import, asset paths, service-worker registration, mobile/TV detection | `interop/ferro_module.rs` (static imports; no async module loading) | 1 |
| `Interop/DomHelper.cs` | DOM imports, global event exports | `interop/dom_helper.rs` | 1 |
| `Interop/CanvasHelper.cs` | Surface create/destroy, size-changed export | `interop/canvas_helper.rs` | 1 |
| `Interop/TimerHelper.cs` | Animation frame import/export | `interop/timer_helper.rs` | 1 |
| `Interop/InputHelper.cs` | Input imports and exports, clipboard, IME element, capture | `interop/input_helper.rs` | 1–3 |
| `Interop/JsCallbackHelper.cs` | Restores the synchronisation context in callbacks | – (no equivalent concept; a re-entrancy guard on exports instead) | – |
| `Interop/GeneralHelpers.cs` | Reflective JS access | folded into `interop/` helpers | 2 |
| `Interop/NavigationHelper.cs` | Back handler, `window.open` | `interop/navigation_helper.rs` | 3 |
| `Interop/ScreenHelper.cs` | Screen imports | `interop/screen_helper.rs` | 3 |
| `Interop/NativeControlHostHelper.cs` | Native control host imports | `interop/native_control_host_helper.rs` | 4 |
| `Interop/StorageHelper.cs` | Storage imports | `interop/storage_helper.rs` | 3 |
| `Interop/StreamHelper.cs` | Stream imports | `interop/stream_helper.rs` | 3 |
| `Rendering/BrowserSurface.cs` | Surface base: size, scaling, compositor | `rendering/browser_surface.rs` | 1 |
| `Rendering/RenderTargetBrowserSurface.cs` | Surface with platform graphics and ready state | `rendering/render_target_browser_surface.rs` | 1 |
| `Rendering/WebRenderTarget.cs` | Render-target base and factory | `rendering/web_render_target.rs` | 1 |
| `Rendering/BrowserSoftwareRenderTarget.cs` | Framebuffer surface with blit | `rendering/browser_software_render_target.rs` | 1 |
| `Rendering/BrowserWebGlRenderTarget.cs` | GL surface and GL context over Emscripten GL | `rendering/browser_web_gl_render_target.rs` (Emscripten only) | 1 |
| `Rendering/BrowserRenderTimer.cs` | Render timer | `rendering/browser_render_timer.rs` | 1 |
| `Rendering/BrowserSharedRenderLoop.cs` | Shared render loop | `rendering/browser_shared_render_loop.rs` | 1 |
| `Rendering/RenderWorker.cs` | Render web worker (threaded mode) | – | – |
| `Storage/BrowserStorageProvider.cs` | Storage provider, items, bookmarks | `storage/browser_storage_provider.rs` | 3 |
| `Storage/BlobReadableStream.cs` | Read stream over a Blob | `storage/blob_readable_stream.rs` | 3 |
| `Storage/WriteableStream.cs` | Write stream over a file handle | `storage/writeable_stream.rs` | 3 |
| `Storage/BrowserLauncher.cs` | Launcher | `storage/browser_launcher.rs` | 3 |
| `..\..\Shared\RawEventGrouping.cs` (linked) | Input grouping for threaded mode | – | – |

### TypeScript and web build

| Upstream file | Purpose | FerroUI counterpart (`webapp/`) | Phase |
|---|---|---|---|
| `webapp/package.json`, `bun.lock` | Dev dependencies, file-system polyfill | `package.json` + lock file | 1 |
| `webapp/build.js` | esbuild: three ESM bundles, minified, source maps | `build.js` | 1 |
| `webapp/tsconfig.json`, `.eslintrc.json` | Compiler and lint settings | same | 1 |
| `webapp/types/dotnet.d.ts` | .NET runtime typings | – | – |
| `modules/avalonia.ts` | Main entry | `modules/ferroui.ts` | 1 |
| `modules/avalonia/jsExports.ts` | Export resolution | `modules/ferroui/ferroExports.ts` | 1 |
| `modules/avalonia/dom.ts` | Host, canvas, global events | `modules/ferroui/dom.ts` | 1 |
| `modules/avalonia/timer.ts` | Animation frames | `modules/ferroui/timer.ts` | 1 |
| `modules/avalonia/singleThreadedDispatcher.ts` | Dispatcher scheduling | `modules/ferroui/singleThreadedDispatcher.ts` | 1 |
| `modules/avalonia/rendering/canvasSurface.ts` | Canvas surface | `modules/ferroui/rendering/canvasSurface.ts` | 1 |
| `modules/avalonia/rendering/resizeHandler.ts` | Resize observation | `modules/ferroui/rendering/resizeHandler.ts` | 1 |
| `modules/avalonia/rendering/renderingMode.ts` | Mode enum | `modules/ferroui/rendering/renderingMode.ts` | 1 |
| `modules/avalonia/rendering/webRenderTarget.ts` | Target base | `modules/ferroui/rendering/webRenderTarget.ts` | 1 |
| `modules/avalonia/rendering/webRenderTargetRegistry.ts` | Target registry, worker transfer | `modules/ferroui/rendering/webRenderTargetRegistry.ts` (no worker branch) | 1 |
| `modules/avalonia/rendering/softwareRenderTarget.ts` | 2D blit | `modules/ferroui/rendering/softwareRenderTarget.ts` | 1 |
| `modules/avalonia/rendering/webGlRenderTarget.ts` | WebGL context and Emscripten registration | `modules/ferroui/rendering/webGlRenderTarget.ts` | 1 |
| `modules/avalonia/input.ts` | Input, clipboard, IME helpers | `modules/ferroui/input.ts` | 1–3 |
| `modules/avalonia/caretHelper.ts` | Caret coordinates | `modules/ferroui/caretHelper.ts` | 2 |
| `modules/avalonia/caniuse.ts` | Capability and device detection | `modules/ferroui/caniuse.ts` | 1 |
| `modules/avalonia/generalHelpers.ts` | Reflective helpers | – | – |
| `modules/avalonia/screens.ts` | Screens | `modules/ferroui/screens.ts` | 3 |
| `modules/avalonia/navigationHelper.ts` | Back handler, open URI | `modules/ferroui/navigationHelper.ts` | 3 |
| `modules/avalonia/nativeControlHost.ts` | DOM attachments | `modules/ferroui/nativeControlHost.ts` | 4 |
| `modules/avalonia/stream.ts` | Stream helpers | `modules/ferroui/stream.ts` | 3 |
| `modules/storage.ts` | Storage entry | `modules/storage.ts` | 3 |
| `modules/storage/storageProvider.ts` | Pickers, bookmarks | `modules/storage/storageProvider.ts` | 3 |
| `modules/storage/storageItem.ts` | Storage item operations | `modules/storage/storageItem.ts` | 3 |
| `modules/storage/indexedDb.ts` | IndexedDB wrapper | `modules/storage/indexedDb.ts` | 3 |
| `modules/avalonia-sw.ts` | Service worker | `modules/ferroui-sw.ts` | 4 |

### Sample

| Upstream file | Purpose | FerroUI counterpart (`samples/ControlCatalog.Browser/`) | Phase |
|---|---|---|---|
| `Program.cs` | Entry: build app, parse options from the query string, start on `"out"` | `program.rs` (ported, with `PreferFileDialogPolyfill`) | 3 |
| `ControlCatalog.Browser.csproj` | WASM SDK project, threads off | `Cargo.toml` | 1 |
| `wwwroot/index.html`, `app.css`, `favicon.ico` | Host page with splash | `index.html`, `app.css`, `favicon.svg` (a neutral placeholder mark instead of the upstream logo) | 1 |
| `wwwroot/main.js` | Runtime bootstrap | `main.js` importing the generated module | 1 |
| `EmbedSample.Browser.cs`, `wwwroot/embed.js` | Native control host demo (iframe, DOM button) | `embed_sample_browser.rs`, `embed.js` | 4 |

## 16. State and handover (2026-10-05)

Done and in the repository: the toolchain spike (B0), core enablement (B1), the backend skeleton (B2), input (B3: pointer, wheel, keyboard, text input and IME, input pane, focus, cursors), services (B4: clipboard, drag and drop, screens, insets, system navigation, launcher), storage (B5) and the native control host with the service worker (B6). Also done: the ControlCatalog host (B7) and the Pages workflow (B8). In progress on a branch: the first size pass (B9a).

### Working on the browser platform from a fresh machine

```
scripts/browser/setup.sh                 # emsdk 6.0.10, Rust 1.90.0 + wasm32-unknown-emscripten, wasm-bindgen CLI (pins in the script and Cargo.toml)
source .tools/env.sh
scripts/build-browser.sh themed_view     # npm ci, type check, lint, bundle, release build, site in target/browser/themed_view
node scripts/browser/tests/themed_view.test.mjs                               # behaviour and key matrix, headless Chrome
scripts/build-browser.sh storage_view && node scripts/browser/tests/storage_view.test.mjs   # storage provider, headless Chrome
scripts/build-browser.sh control-catalog-browser
node scripts/browser/tests/control_catalog.test.mjs [--screenshot out.png]    # the ControlCatalog site: rendering, resize, drawer, navigation, click, typing
node scripts/browser/capture.mjs target/browser/themed_view out.png [--query "?RenderingMode=Software2D"] [--scale 2]
(cd src/Browser/FerroUI.Browser/webapp && npm run test:pixels)                # software blit, straight alpha
cargo test -p ferroui-browser            # desktop unit tests of the backend (no browser needed)
cargo check --locked --target wasm32-unknown-emscripten -p ferroui-browser --examples
```

`scripts/browser/harness.mjs` is the test library: it serves a site, drives Chrome or Chromium over the DevTools protocol (real mouse, wheel and key events), reads pixels and evaluates expressions. It finds the browser through `CHROME`, the usual install paths and Playwright's directories. The example exports `themedViewState()` (a line of `name=value` pairs) and the host page exposes the module as `globalThis.themedView`; the ControlCatalog host exports `catalogState()` and exposes `globalThis.controlCatalog`; give a new application the same kind of hook rather than testing through pixels alone.

Rules that keep the boundary sound (section 5): imports by `raw_module = "./ferroui.js"` with `js_namespace`/`js_name`, typed getters on `extern` types, flat exports `<Class>_<Method>` listed in `ferroExports.ts` (every name on both sides), no `js-sys`/`web-sys`/closures, synchronous answers. Before building for a new target or feature set run `cargo tree -e features` on `ferroui-skia`: an unpublished Skia feature combination silently starts a source build.

### Remaining stages

| Stage | Upstream files | What is known |
|---|---|---|
| B4 services (done) | `ClipboardImpl.cs`, `BrowserClipboardDataTransfer*.cs`, `BrowserDragDataTransfer*.cs`, `BrowserDataFormatHelper.cs`, `BrowserDataTransferHelper.cs`, `BrowserInsetsManager.cs`, `BrowserScreens.cs`, `BrowserSystemNavigationManager.cs`, `Storage/BrowserLauncher.cs`, `Interop/NavigationHelper.cs`, `Interop/ScreenHelper.cs`; `screens.ts`, `navigationHelper.ts`, the clipboard and drag functions of `input.ts` | Asynchronous calls complete through `interop/completion_helper.rs` and `completionHelper.ts` (section 14). The top-level offers the clipboard, the insets manager and the launcher; the platform registers the screens (`DomHelper_ScreensChanged` is back) and the system navigation manager. The paste-event fallback of `initializeBackgroundHandlers` is in place. The file data format and the storage files of a drop or paste came with B5 (see its row). The application prefix of custom formats is `application/frn-fmt.`. `scripts/browser/tests/themed_view.test.mjs` drives copy, cut and paste through a text box (also through the paste event), a denied read, a drop of text, the screen and the safe area, the detailed screens, the launcher and the back navigation |
| B5 storage (done) | `Storage/BrowserStorageProvider.cs`, `BlobReadableStream.cs`, `WriteableStream.cs`, `Interop/StorageHelper.cs`, `Interop/StreamHelper.cs`; `storage.ts`, `storage/*.ts`, `stream.ts` | Done (merged). `storage.js` is a second esbuild entry, imported on first use by `importStorage` of `ferroui.ts` and reached from Rust through `StorageModule.module`; the polyfill is pinned to `d43ad84` (NOTICE). Asynchronous imports return their promise, awaited with `await_promise` of `interop/completion_helper.rs` (B4), the one completion mechanism of the boundary. The sync `Read`/`Write` of the contracts are bridged as section 14 says; `StorageItem.isSameEntry` (not upstream) matches a pending close to its file. `IStorageItem::as_any` lets the provider recognise its items (start locations, move). `scripts/browser/tests/storage_view.test.mjs` (16 checks) stubs the pickers with origin-private handles, drops a file on the view and drives the polyfill's file input and download. The file seams of B4 are closed: the core has the file data format (`DataFormat::file`, `try_get_file(s)` of the data transfers), the page format `Files` maps to it, `input.ts` wraps a `File` into a storage item and a file value is a `JsStorageFile`, and the drag handler imports the storage module. The service worker for polyfill saves came with B6 (see its row). Errors of operations whose contract has no error channel become `None` or `()` (section 14) |
| B6 native control host, service worker (done) | `BrowserNativeControlHost.cs`, `Interop/NativeControlHostHelper.cs`, `nativeControlHost.ts`, `avalonia-sw.ts` with its registration (`Interop/AvaloniaModule.cs`, `registerServiceWorker` of `avalonia.ts`, `WindowingPlatform.Register`), `EmbedSample.Browser.cs` and `wwwroot/embed.js` | `browser_native_control_host.rs` and `interop/native_control_host_helper.rs` over `ferroui/nativeControlHost.ts` (1:1); the top-level offers the host as its `INativeControlHostImpl` feature, over the `nativeHost` element of `createFerroHost`. The service worker is `webapp/modules/ferroui-sw.ts` (1:1), a third esbuild entry, copied by `scripts/build-browser.sh` to the root of the site and registered by `BrowserWindowingPlatform::register` when `register_ferro_service_worker` is set (scope `ferro_service_worker_scope`). The catalog host sets `EmbedSampleWeb` (`samples/ControlCatalog.Browser/embed_sample_browser.rs`) as the implementation of `EmbedSample` after setup, as `Program.cs` does. Tests: `themed_view.test.mjs` attaches, resizes, moves, hides, shows, detaches and re-attaches a native control through `themedViewNativeHost` and reads the element in the DOM; `storage_view.test.mjs` checks that no worker is registered by default (the polyfill saves through a blob link) and that with `?RegisterServiceWorker=true` the worker is active at the root and serves the polyfill's download; `control_catalog.test.mjs` opens the Native Embed page, clicks the button of `embed.js` with real pointer events and hides a sample with its check box. Two upstream defects were fixed on the way (section 14): the service worker never asked for the first chunk, so a save through it never started, and the pointer handlers of the view took the events of native controls, so a native button never got its click. Not done: keyboard events of a focused native control still reach the handlers of the view, as upstream |
| B9b cross-browser | - | Only Chrome 154 (headless, ANGLE on Metal) has run anything. Unverified: Firefox, Safari/WebKit, mobile, WebGL1, the `failIfMajorPerformanceCaveat` fallback to the software path, `ResizeObserver` without `devicePixelContentBoxSize` (Safari), `scheduler.postTask` fallbacks, IME on real input methods, touch and pen hardware. The harness speaks the Chrome DevTools protocol only |

Known gaps inside the finished stages are listed in the stage READMEs of the deliveries and in section 14; the largest are dropped and pasted files (they wait for the storage of B5), and that nothing but headless Chrome has been driven.

## 17. Sources

- rust-skia README (Emscripten support, prebuilt targets): https://github.com/rust-skia/rust-skia
- skia-safe on crates.io (0.153.3, 2026-09-04): https://crates.io/crates/skia-safe
- harfbuzz-sys on crates.io (0.8.0) and build script: https://crates.io/crates/harfbuzz-sys, https://github.com/servo/rust-harfbuzz
- wasm-bindgen guide, Emscripten target: https://wasm-bindgen.github.io/wasm-bindgen/reference/emscripten.html
- wgpu Emscripten issue and PR: https://github.com/gfx-rs/wgpu/issues/10274, https://github.com/gfx-rs/wgpu/pull/10515
- Vello README: https://github.com/linebender/vello
- HarfRust releases; rustybuzz archival: https://github.com/harfbuzz/harfrust/releases, https://github.com/harfbuzz/rustybuzz
- Local Font Access API: https://developer.mozilla.org/en-US/docs/Web/API/Local_Font_Access_API
- WebGPU overview: https://developer.chrome.com/docs/web-platform/webgpu/overview

## 18. Module size (2026-10-05, measured again 2026-10-06)

The ControlCatalog site is published only while its WebAssembly module is within a gzip budget, 12 MB when this section was written and 14 MB since the themes load from compiled markup (stage E4 of the XAML compiler: the catalog module is then 43.85 MB raw and 11.42 MB with gzip, against 31.87 MB and 9.45 MB below). This section records what the modules are made of, what was changed to bring them down, and what each change saved. Everything here is **[M]**: measured on Linux with Rust 1.90.0, Emscripten 6.0.10 and wasm-bindgen 0.2.129. Sizes are in MB of 1,000,000 bytes; gzip is level 9 of Node's zlib, as `scripts/browser/module-sizes.mjs` of the Pages budget computes it (GitHub Pages compresses with gzip); the attribution tables use Python's zlib, which differs by less than 1 %. How it was measured: sizes of the files of a site with `scripts/browser/module-sizes.mjs`; the attribution with `scripts/browser/wasm-size-report.py`; start-up with `scripts/browser/first-frame.mjs` (headless Chromium 141 with WebGL on SwiftShader, local server, 4-core container, five loads per rendering mode with a fresh browser profile each; the figure is the time from navigation to the first frame on screen, WebGL2 / Software2D); behaviour with the tests of `scripts/browser/tests/`, which drive the page through `scripts/browser/harness.mjs`. The step-by-step measurements of 2026-10-05 were taken with the end-to-end test of the catalog host as it was then (the time from navigation until the splash screen closed and the view had its canvas, two loads per mode) and with a first-frame CPU profile; they are comparable with each other, not with the figures of 2026-10-06, which come from a different container.

### What the module is made of

`scripts/browser/wasm-size-report.py` attributes a module: section sizes, embedded files in the data section, and, for a module linked with its name section (`-C link-arg=--profiling-funcs`), the code by origin and by generic family. The `themed_view` example before this pass (release profile: thin LTO, 4 code generation units, opt-level 3; 49.41 MB, 10.46 MB gzip):

| Part | MB | MB gzip (alone) |
|---|---:|---:|
| Code section | 44.07 | 8.35 |
| of which Rust `ferroui_base` (property system, bindings, value type registry, metadata) | 24.48 | 2.61 |
| of which Rust `core` (closures `FnOnce::call_once`, iterators, formatting) | 4.96 | 1.29 |
| of which Rust `ferroui_controls` | 3.60 | 0.81 |
| of which drop glue (`drop_in_place`) of all types | 1.85 | 0.15 |
| of which XAML pipeline (`ferroui_markup_xaml_loader`, `xamlx`, `ferroui_markup_xaml`, `roxmltree`) | 2.50 | 0.60 |
| of which Skia (Ganesh, raster, SkSL, FreeType) | 3.17 | 1.20 |
| of which HarfBuzz | 0.50 | 0.15 |
| of which image codecs and zlib (libjpeg-turbo, libpng, wuffs) | 0.32 | 0.09 |
| of which C and C++ runtime (libc, libc++, Emscripten) and other C | 1.07 | 0.36 |
| Data section | 4.99 | 1.77 |
| of which the six Inter fonts | 1.88 | 0.92 |
| of which embedded theme markup (Fluent) | 0.71 | 0.11 |
| of which other constant data (strings, tables, panic locations, vtables) | 2.40 | 0.74 |
| Element section (function table) | 0.26 | 0.21 |

The "alone" column compresses the bytes of one row by themselves, so the rows do not add up to the gzip size of the section; it shows how much of a row is repetition.

Largest generic families (release profile; instances, MB raw):

| Family | Instances | MB | MB gzip (alone) |
|---|---:|---:|---:|
| closures `FnOnce::call_once` (value type registry conversions, markup metadata tables, class vtables) | 12,566 | 3.53 | 0.96 |
| `drop_in_place<_>` | 4,437 | 1.85 | 0.15 |
| `ValueTypes::register_object<T>` | 348 | 1.85 | 0.06 |
| `BindingEntry<T>::set_value` | 257 | 1.01 | 0.11 |
| typed observers `on_next` | 1,827 | 0.87 | 0.06 |
| `StyledProperty<T>::from_untyped` | 257 | 0.75 | 0.07 |
| `route_bind` of `StyledProperty<T>` / `DirectProperty<T>` | 316 | 0.73 | 0.03 |
| `markup_types::register_value_types` of `ferroui_base` (one function: the generated by-name value type table) | 1 | 0.73 | 0.03 |
| `MarkupArguments::next<T>` | 302 | 0.72 | 0.05 |
| `register_class::init` (class registration, metadata) | 342 | 0.51 | 0.10 |

What the measurement says:

- **Monomorphised framework code is the raw size, but not the gzip size.** The property system is instantiated per value type (257 `StyledProperty<T>` routes, 348 handle types in the value type registry); each instance is a near copy of the others, so a family of 1.85 MB compresses to 0.06 MB. Outlining generic code therefore shrinks the raw module (download after decompression, compile time and memory of the browser) much more than the gzip transfer.
- **The by-name metadata tables** (the value type registry with its conversion closures, the markup type tables and argument readers, class and property registration: 6.7 MB raw of code in the release build) are needed by both measured applications: `themed_view` and the catalog load their theme and page markup through the run-time XAML loader, which looks every type and member up by name. Gating them behind the loader saves nothing for these applications; it pays off once markup is compiled ahead of time (`xaml.md`) and an application no longer calls the crate-wide `register_types()`.
- **Skia is 3.2 MB raw, 1.2 MB gzip**, and the linker already drops what the framework does not reference: no code of the PDF or SVG back ends is in the module; the JPEG and PNG encoders (0.13 MB) are, because bitmaps can be saved. Its feature set per target is fixed by the published binaries (section 3); a different feature set means a source build of Skia, which the project does not do.
- **No ICU or similar data is linked.** The Unicode property tables of the text formatting code are the framework's own (`ferroui_base::media::text_formatting::unicode`, generated by `scripts/convert-unicode-tries.py`) and are part of the "other constant data" row.
- **Panics and formatting** are small: the code of `core::fmt` and `alloc::fmt` is 0.08 MB raw and the panic machinery less than 0.01 MB; panic messages and locations are part of the 2.4 MB of other constant data. The cost of unwinding is elsewhere (next item).
- **Exceptions cost speed, not only size.** Rust on the Emscripten target with the pinned toolchain unwinds with JavaScript exceptions: every call that may unwind goes through an `invoke_*` function in JavaScript. A CPU profile of the start-up of `themed_view` at opt-level "z" without the change below spent 6.6 s in `WebAssembly.Table.get`, 2.1 s in the table lookup function, 2.4 s in wasm-to-JavaScript transitions and 1.6 s in JavaScript-to-wasm transitions, against 6.4 s in WebAssembly code. WebAssembly exception handling for Rust on this target is an unstable option of rustc (`-Z emscripten-wasm-eh`) that the pinned stable toolchain does not offer; it was not tried.

### Changes

| Change | Where |
|---|---|
| Type-independent parts of generic functions compiled once: value type registry insertions, binding notification errors, markup value conversion, property registry, the error paths of the typed property routes. These are the patches of the desktop size pass (`desktop-performance.md`), which landed on `main` with it; the step-by-step measurements below include them as a step, the measurements of 2026-10-06 have them on both sides | `ferroui-base` (desktop size pass) |
| Profile `browser`: `lto = "fat"` (thin LTO since stage E4 of the XAML compiler, see "Build memory" below), `codegen-units = 1`, `opt-level = "z"`, with `opt-level = 3` for `xamlx`, `ferroui-markup-xaml-loader`, `ferroui-markup-xaml`, `ferroui-markup` and `roxmltree`; `scripts/build-browser.sh` builds with it. rustc passes the optimisation level to `em++`, which runs `wasm-opt -Oz` on the linked module; the module has no name section and no debug information, as before | `Cargo.toml`, `scripts/build-browser.sh` |
| The function table lookup with its cache (`wasm_table_mirror.js`, linked with `--js-library`): Emscripten leaves the cache out of `-Os` and `-Oz` links, and every `invoke_*` looks its callee up. The library applies only to those links (`#if SHRINK_LEVEL > 0`): the dev profile keeps the runtime's own cache with its assertion. It is written against the runtime of the pinned Emscripten 6.0.10, in which the table entries are written only through `setWasmTableEntry` (replaced as well) and the table only grows for dynamic linking, which is not used | `src/Browser/FerroUI.Browser/emscripten/`, `.cargo/config.toml` |
| `-sENVIRONMENT=web`: the script of the module is linked for web pages only | `.cargo/config.toml` |
| Asset bundles: an application can ship assets as a file next to the module (`register_asset_bundle` of the asset registry, `registerAssetBundle` exported to the host page, `$OUT_DIR/browser-site/` of a build script copied into the site by `scripts/build-browser.sh`). The feature `separate-assets` of `control-catalog` writes its pictures and fonts (everything but the markup documents) to `control-catalog.assets`; the browser host enables it and its page fetches the bundle while the module compiles | `ferroui-base`, `ferroui-browser`, `control-catalog`, the catalog host |

### Before and after on `main` of 2026-10-06

Measured after the pass was rebased onto `main` at `18155e5`, which carries the catalog host (`samples/ControlCatalog.Browser`), the storage, services and dialogs of the browser backend, and the desktop size pass: the outlining patches, and a release profile with fat LTO in one code generation unit at opt-level 3. "Before" is `main` built with that release profile; "after" is this pass: the `browser` profile, and the catalog host with the feature `separate-assets` fetching `control-catalog.assets`. First frame: median and range of five loads with `first-frame.mjs`, WebGL2 / Software2D; for `themed_view` the medians of three such rounds.

| Module | Before MB | Before MB gzip | After MB | After MB gzip | First frame before (s) | First frame after (s) |
|---|---:|---:|---:|---:|---:|---:|
| `themed_view.wasm` | 41.02 | 9.85 | **25.89** | **8.25** | 9.05 to 9.07 / 8.06 to 9.31 | 9.70 to 10.00 / 9.11 to 9.50 |
| `control_catalog_browser.wasm` | 75.29 | 31.63 | **31.87** | **9.45** | 14.9 (13.5 to 16.0) / 13.3 (12.4 to 14.8) | 14.6 (14.2 to 15.7) / 14.2 (13.6 to 15.2) |
| `control-catalog.assets` (new, next to the module) | | | 23.54 | 20.18 | | |
| script of the module (`control-catalog-browser.js`) | 0.22 | 0.04 | 0.22 | 0.04 | | |
| the catalog site as a whole (without source maps) | 75.57 | 31.70 | 55.69 | 29.69 | | |

The catalog module is within the 12 MB budget with 2.55 MB to spare; on this base the `browser` profile takes 15.13 MB raw and 1.60 MB gzip off `themed_view`; of the 22.18 MB gzip the catalog module loses, 20.18 MB move into the asset bundle and about 2.0 MB come from the profile. Against this baseline the first frame of `themed_view` is 0.7 to 0.9 s (8 to 10 %) later: the release profile now has the same fat LTO and one code generation unit, so opt-level "z" outside the XAML pipeline is what remains, and the run-time loader also calls into the property system and the controls. The catalog differs by -0.3 s and +0.9 s, within the spread of its loads. The behaviour tests (`themed_view.test.mjs`, 26 checks; `control_catalog.test.mjs`, 7 checks, including that no file of the site, the bundle included, carries a brand asset that `PlaceholderAssets` replaces) pass on both builds.

### Step by step (`themed_view`, `main` of 2026-10-05)

The contribution of each change was measured on the earlier base (`main` at `58ad3cf`), one build per step:

| Build | MB | MB gzip | Start-up (s) |
|---|---:|---:|---:|
| Before: release profile (thin LTO, 4 units, opt-level 3) | 49.41 | 10.46 | 7.7 to 8.6 / 6.8 to 7.4 |
| Outlining patches, release profile | 44.47 | 10.05 | 7.8 to 8.2 / 6.5 to 7.3 |
| Outlining, fat LTO, 1 unit, opt-level "s" everywhere | 33.74 | 8.41 | 11.5 / 9.9 to 10.3 |
| Outlining, fat LTO, 1 unit, opt-level "z" everywhere | 26.21 | 7.79 | 15.6 to 17.0 / 15.3 to 16.2 |
| the same with the table lookup cache (patched into the script of the module by hand for the measurement) | 26.21 | 7.79 | 11.9 / 10.7 to 11.0 |
| Profile `browser` (opt-level 3 for the XAML pipeline) with the table lookup cache | 24.98 | 7.94 | 7.6 to 7.8 / 6.8 to 7.0 |

What each change saved on this module: the outlining patches 4.94 MB raw and 0.41 MB gzip; the size profile a further 19.49 MB raw and 2.11 MB gzip (keeping the XAML pipeline at opt-level 3 costs 0.15 MB gzip of that and saves 1.23 MB raw); the table lookup cache nothing in size (about 150 bytes of script) and 3.5 to 5 s of start-up at opt-level "z". `-sENVIRONMENT=web` (taken over from an earlier attempt at this pass) changes only the script of the module (about 2 kB less). The rendered pixels are the same in every configuration (the colour counts of the end-to-end test are identical).

### Step by step (ControlCatalog, `main` of 2026-10-05)

| Build | Module MB | Module MB gzip | Other files of the site, MB gzip | Start-up (s) |
|---|---:|---:|---:|---:|
| Before: `browser-catalog` with `main` merged, release profile, assets embedded | 85.83 | 32.39 | 0.05 | 11.8 / 10.6 |
| This pass merged, profile `browser`, assets embedded | 54.44 | 29.34 | 0.05 | not measured |
| This pass merged, profile `browser`, pictures and fonts in `control-catalog.assets` | 30.98 | 9.15 | 20.23 (the bundle: 23.54 MB raw, 20.18 MB gzip) | 11.3 to 11.5 / 10.5 |

What each change saved on this module: the outlining patches and the size profile together 31.39 MB raw and 3.05 MB gzip; the asset bundle moves 23.46 MB raw and 20.19 MB gzip out of the module into a file of its own.

The site as a whole is still 29.7 MB with gzip (32.9 MB before, on `main` of 2026-10-06): the pictures (JPEG, already compressed) and fonts of the sample are 20.2 MB of it, and the bundle moves them out of the module without making them smaller. The page fetches the bundle while the module is compiled, so the start-up did not get longer; the application starts when both are there. The screenshots of the main view before and after are the same, except in the banner picture of the home page, whose pixels differ by up to 6 levels between two runs of the same build.

### Trade-offs

- **Start-up**: against the release profile of `main` of 2026-10-06 (fat LTO, one unit, opt-level 3) the first frame of `themed_view` is 0.7 to 0.9 s (8 to 10 %) later, the catalog's within noise. Against the earlier release profile (thin LTO, four units) no difference was measurable. Opt-level "z" for the whole module doubled the start-up of `themed_view` (7.5 s to 16 s); the table lookup cache and opt-level 3 for the XAML pipeline crates bring most of it back. Opt-level "s" was not adopted: it is larger than "z" and, without the remedies, also slower to start.
- **Run-time speed** after start-up is not measured (no browser benchmark of layout and rendering exists). Code outside the XAML pipeline runs at opt-level "z", which inlines less than the release profile; the desktop measurements (`desktop-performance.md`) put the CPU cost of opt-level "s" at about 19 %.
- **Build time**: fat LTO in one code generation unit serialises code generation (the profile has thin LTO since the themes load from compiled markup; see "Build memory"). Full builds of `themed_view` with the `browser` profile took 11.5 to 17.7 minutes in this container (4 cores), a full build of the catalog 15 minutes, a rebuild of the catalog crate and its link 10 minutes; the release profile used on the desktop is unchanged.
- **Memory of the page**: the asset bundle is copied once into the module memory (24 MB for the catalog) and its views stay registered for the life of the page, as embedded assets would.

### Build memory (2026-10-06)

Stage E4 of the XAML compiler added the compiled theme markup (`compiled_xaml.rs` of `ferroui-themes-fluent`, 9.1 MB of Rust, and `ferroui-themes-simple`, 4.9 MB). With fat LTO the release build of the catalog module no longer fitted in the 16 GB of a GitHub runner: it needed about 15.5 GB with swap, and was killed at 13.9 GB in a 15 GB container without swap. **[M]**, measured as the peak resident memory of the build's process tree, sampled every 0.5 s, in a 4-core container. The figures are for the whole catalog build, from a clean state for the workspace crates.

Where the memory went, with fat LTO:

- With LTO, cargo builds the dependencies with `-C linker-plugin-lto`, so their rlibs carry only bitcode. All the machine code of the program is generated in the rustc process of the final crate (`control_catalog_browser`), in one LLVM module. Its memory rose steadily from 5.5 GB to 13 GB over four minutes of code generation, the IR of the whole program plus the generated code collected for the one object file. That rustc process holds the peak. The rustc processes of the crates themselves stay under 4.5 GB: `ferroui-controls` 4.5 GB, `ferroui-base` 2.9 GB, `ferroui-themes-fluent` 1.9 GB, `ferroui-themes-simple` 1.2 GB.
- On top of that come the large generated functions. `ferroui-themes-fluent` compiled by itself to a WebAssembly object takes 6.1 GB and 7 minutes, against 1.9 GB as bitcode. About 3 minutes of that is `populate_fluenttheme_xaml`: 1,371 deferred resources in one function, whose code generation (WebAssembly Fix Irreducible Control Flow, Register and CFG Stackify) adds several GB for as long as it runs. The shape of the generated code is a matter for the emitter; the profile does not depend on it.
- Linking happens after rustc exits, so its peaks do not add up with rustc's: `wasm-ld` 0.8 GB, `wasm-bindgen` (run by `emcc` for `-sWASM_BINDGEN`) 4.2 to 5.9 GB, `wasm-opt` 1.6 to 1.8 GB, `wasm-metadce` 0.9 GB.
- No debug information is generated. The profile has `debug = 0`, cargo passes `-C strip=debuginfo`, and rustc then passes `-g0` to `em++`. `debug`/`strip` settings change nothing here.

The catalog module with each setting of the `browser` profile (opt-level "z", the XAML crates at 3, everything else unchanged):

| `lto`, `codegen-units` | Peak memory of the build | Wall time | Module MB | Module MB gzip |
|---|---:|---:|---:|---:|
| fat, 1 (before) | over 13.9 GB, killed (about 15.5 GB with swap) | — | 43.85 | 11.42 |
| thin, 16 (the default) | 6.2 GB | 14.2 min | 47.60 | 12.28 |
| **thin, 1 (adopted)** | **8.0 GB** (final rustc 6.4 GB) | 17.0 min | 44.63 | 11.58 |

Thin LTO keeps one module per code generation unit, generates their code separately (four at a time on four cores) and writes one object each, so no single module holds the whole program. Fat LTO merges everything into one module however the crates are split, so `codegen-units` or `opt-level` for the theme crates do not change its code generation. With one unit per crate, thin LTO costs 0.78 MB raw and 0.16 MB gzip (1.4 %) against fat LTO and leaves 2.4 MB of the 14 MB Pages budget. Sixteen units save 1.8 GB more but cost 0.86 MB gzip. The peak of 8.0 GB is reached while `ferroui-controls` and other crates compile in parallel, which leaves about 8 GB of headroom on a 16 GB runner. `themed_view` with the same profile peaks at 6.1 GB (34.34 MB, 9.64 MB gzip). Both sites pass their browser tests (`control_catalog.test.mjs`, 8 checks; `themed_view.test.mjs`, 28 checks). The desktop `release` profile keeps fat LTO.

### Not done, and why

- `panic = "abort"`: the framework relies on unwinding (`catch_unwind` in the render loop and the dispatcher, panics of dispatcher operations re-raised in their awaiters; see `desktop-performance.md`, P2 item 6), and the pinned toolchain builds the target only with `unwind` (section 3).
- Splitting Skia features: forbidden by the binary-only Skia rule (section 3); the linker already drops the unused parts.
- Compressing embedded resources inside the module: the fonts and markup are compressed by the transfer encoding already, and the pictures are JPEG and PNG; a second compression would cost a decompressor and start-up time for no transfer gain.
- Further outlining of the property store (`BindingEntry<T>::set_value`, `EffectiveValue<T>`, `StyledProperty<T>::from_untyped`, about 4 MB raw): hot paths of a port of generic upstream code, each split needs the property system tests and keeps the borrow and re-entrancy order; it would shrink the raw module, while its gzip effect is small (the families compress to a few percent). Proposed as a follow-up together with one shared instantiation for reference-like value types (`desktop-performance.md`, items 10 and 11).

## 19. Start-up time of the ControlCatalog site (2026-10-06)

Measured **[M]** on the published site (`https://wieslawsoltes.github.io/FerroUI/`, deployed 2026-10-06 11:15 UTC from `main`), whose files were downloaded and served locally with `first-frame.mjs --encoding gzip` (the bytes GitHub Pages transfers), in headless Chromium 141 with WebGL on SwiftShader, 4-core container. Another build was running on the machine during most measurements, so absolute CPU times vary by about ±0.5 s between loads; the comparisons below were interleaved load by load.

### What GitHub Pages serves

| File | Raw MB | Transferred (gzip) MB | Headers |
|---|---:|---:|---|
| `control_catalog_browser.wasm` | 31.88 | 9.53 | `application/wasm`, `cache-control: max-age=600`, ETag |
| `control-catalog.assets` | 23.54 | 20.20 | `application/octet-stream`, same caching |
| scripts, page, styles | 0.29 | 0.07 | same caching |

The media type of the module is right, and the script of the module compiles it with `WebAssembly.instantiateStreaming`; the compilation overlaps the download and is done 30 to 50 ms after the last byte (V8 compiles lazily, function by function, as they are first called). The module has no name section. Repeat visits within ten minutes come from the HTTP cache; after that each file is revalidated (a `304` for an unchanged file). Pages does not let the site set its own cache headers.

### Where the time goes (before)

Medians of three loads, milliseconds from the start of navigation:

| Phase | Unthrottled | 50 Mbit/s, 40 ms |
|---|---:|---:|
| page, `main.js`, `ferroui.js`, script of the module downloaded | 52 | 176 |
| module downloaded (streaming compilation running alongside) | 328 | 3,577 |
| module compiled | 375 | 3,610 |
| module instantiated | 419 | 3,649 |
| asset bundle downloaded (the application waits for it) | 251 | 5,489 |
| `runMain` (set-up, application initialisation, main view): about 7,700 ms | 428 to 8,129 | 5,500 to 13,200 |
| first layout and render, first draw | 8,356 | 13,283 |
| first frame on screen | **8,475** | **13,388** |

The splash screen of the host page is part of the HTML and is painted before any script runs, as upstream's; it stays until the first frame.

The 7.7 s inside `runMain`, from a CPU profile of the start-up (samples inside the window of `runMain`; `scripts/browser/first-frame.mjs --cpu-profile`):

| Category | ms | Share |
|---|---:|---:|
| WebAssembly code | 3,017 | 39 % |
| wasm-to-JavaScript and JavaScript-to-wasm transitions | 3,124 | 41 % |
| the `invoke_*` functions of the runtime, the table lookup, `stackSave`/`stackRestore` | 1,457 | 19 % |
| other (JavaScript, garbage collection, WebGL glue) | 103 | 1 % |

The transitions and the `invoke_*` glue (4.58 s, 60 %) are the cost of unwinding with JavaScript exceptions (section 18): with Rust 1.90, the pinned toolchain, every call that may unwind out of a function with clean-up code leaves the module for an `invoke_*` function in JavaScript, which calls back into the module through the function table. The work itself is 3.0 s.

By phase, from a CPU profile of the same code linked with its name section (`cargo rustc --profile browser -p control-catalog-browser --bin control-catalog-browser --target wasm32-unknown-emscripten -- -Clink-arg=--profiling-funcs`: the code section differs from the published one by 172 bytes), unthrottled, the module and the bundle served locally; wall clock from the start of navigation:

| Phase | From (ms) | To (ms) | Wall ms |
|---|---:|---:|---:|
| module and bundle downloaded, module compiled and instantiated | 0 | 579 | 579 |
| platform set-up and type registration | 585 | 641 | 56 |
| `App.xaml` with `CustomThemes.xaml` (run-time loader) | 641 | 1,257 | 616 |
| **`FluentTheme` (`App::resource("FluentTheme")`, a deferred resource, built through the run-time loader)** | 1,259 | 5,777 | **4,518** |
| **`SimpleTheme` (`App::resource("SimpleTheme")`, likewise)** | 5,779 | 8,371 | **2,592** |
| the rest of `runMain`, with the main view and the home page (`App::create_main_view_host`, 133 ms) | 8,371 | 8,628 | 257 |
| first layout and render (0.2 s of it compiling WebGL shaders on SwiftShader) | 8,628 | 9,361 | 733 |
| first frame on screen | | 9,476 | |

So **loading the two themes at run time is 7.1 s of the 9.5 s to the first frame** on a fast network (75 %), and of the 8.0 s of `runMain` (88 %). Upstream builds both as well (`App.Initialize` reads both resources), but from compiled XAML. Inside the two loads, the samples split as follows:

| Stage of the run-time loader | `FluentTheme` | `SimpleTheme` |
|---|---:|---:|
| parse | 2 % | 2 % |
| transform (the XamlX transformer passes; mostly the tree walk of the visitors, `visit_node`, `visit`, `ContextXamlAstVisitor::visit`) | 89 % | 93 % |
| build and populate the objects (`run_build`, `run_populate`) | 9 % | 4 % |

Compiled themes (stage E4, `xaml.md`) do the parse and transform at build time, so they should remove about 90 % of these 7.1 s. The rest, building the style objects, remains as compiled code. The transform does not show a quadratic or redundant step: it is upstream's design of one pass over the tree per transformer, and each recursive visit pays the unwinding overhead above.

### Changes

| Change | Saved |
|---|---|
| The host page preloads `main.js`, the scripts it imports, the module and the asset bundle (`<link rel="modulepreload">`, `<link rel="preload" as="fetch" crossorigin>`), as upstream's page preloads the boot resources of its runtime. The downloads start with the page instead of after its scripts have been fetched and evaluated. `control_catalog.test.mjs` checks that each file is downloaded once, through its preload | 50 Mbit/s, 40 ms: the application starts 180 ms earlier (`runMain` start, median of three interleaved pairs: 5,485 to 5,305 ms); unthrottled: within noise |
| `first-frame.mjs --throttle --phases`, and performance marks in `main.js` (module instantiated, asset bundle downloaded, `runMain` start and end) | measurement only |

### What remains, and what to do about it

- **Unwinding with JavaScript exceptions, about 60 % of the CPU time of the start-up.** Rust 1.93.0 switched `wasm32-unknown-emscripten` to WebAssembly exception handling by default (and 1.98.0 removes the JavaScript variant). Recommended next step: move the browser toolchain from 1.90.0 to a release from 1.93 on, link with `-fwasm-exceptions` and compile the C and C++ code of the module (HarfBuzz through `EMCC_CFLAGS`) with it as well. Check the prebuilt Skia library links with it, since Skia is never built from source. This removes the `invoke_*` functions, their transitions and the table lookup library of section 18, and probably makes the module smaller. Expected effect, not measured: most of the 4.6 s.
- **Run-time theme loading, 7.1 s, about 90 % of it the run-time compilation.** Stage E4 (compiled themes). The theme loads also shrink with the exception change above, and so does the run-time loading of the catalog's own documents (`App.xaml` 0.6 s, the pages), which stays until the documents of the sample are compiled as well.
- **The asset bundle, 20.2 MB transferred, is on the critical path.** At 50 Mbit/s it arrives 1.9 s after the module, and the application waits for it. It is not split into a start-up part and a deferred part, because the asset loader is synchronous, as upstream's is: a page created before a deferred part arrived would not find its pictures, and making the pages wait would change the catalog. Upstream does not defer them either: they are embedded resources of the sample's assembly, which the .NET runtime downloads with all other boot resources before `Main` runs. The start-up itself needs only `icon.ico` (from `App.xaml`) and the six PNG files of the home page; the 79 photographs (18.1 MB) and the fonts (5.3 MB, 4.5 MB of it one CJK font) belong to other pages. Reducing the bundle is part of the size work that follows.
  Since done in the host of the sample, without changing the asset loader: see "Assets on demand" below.
- **Not adopted:** a service worker cache of the module and the bundle. Within ten minutes the HTTP cache already serves repeat visits, and after that a revalidation costs one round trip per file. A service worker would save only those round trips, not the CPU time that dominates, and would add a second cache that has to be versioned against deployments.
- **Not measured:** the first frame in the WebGL2 path spends about 0.2 s compiling shaders on SwiftShader; on a real GPU this differs.

### Assets on demand (2026-10-06)

The catalog's build script splits its assets by the pages that use them, and its browser host fetches the bundles of a page before the catalog creates the page (section 14, row `ControlCatalog.Browser` host). Bundles as written by `scripts/build-browser.sh control-catalog-browser` (`node scripts/browser/module-sizes.mjs` prints this table from `control-catalog.assets.json`):

| Bundle | Raw | gzip -9 | Assets | Loaded for |
|---|---:|---:|---:|---|
| `control-catalog.assets` | 0.44 MB | 0.39 MB | 10 | start-up |
| `control-catalog.ContainerQueryPage.assets` | 0.52 MB | 0.51 MB | 7 | Container Queries |
| `control-catalog.CursorPage.assets` | 0.6 kB | 0.6 kB | 1 | Cursor |
| `control-catalog.DrawerDemoPage.assets` | 0.36 MB | 0.35 MB | 9 | DrawerPage |
| `control-catalog.WenQuanYiMicroHei-01.assets` | 4.51 MB | 2.05 MB | 1 | TextBox, TextBlock |
| `control-catalog.shared.Fonts-SourceSansPro-Regular.assets` | 0.79 MB | 0.37 MB | 4 | TextBox, TextBlock |
| `control-catalog.shared.ModernApp-gallery_alpine.assets` | 0.19 MB | 0.18 MB | 6 | CarouselPage, DrawerPage |
| `control-catalog.shared.Sanctuary-main_deep_forest.assets` | 3.10 MB | 3.04 MB | 7 | PipsPager, CarouselPage |
| `control-catalog.shared.delicate-arch-896885_640.assets` | 95.9 kB | 95.8 kB | 1 | 13 pages |
| `control-catalog.shared.hirsch-899118_640.assets` | 0.22 MB | 0.22 MB | 1 | 8 pages |
| `control-catalog.shared.maple-leaf-888807_640.assets` | 0.11 MB | 0.11 MB | 1 | 9 pages |
| `control-catalog.unreached.assets` | 13.21 MB | 12.87 MB | 46 | no page (the demos of `NavigationDemoPage`, whose class is not ported; prefetched last) |
| all 12 together | 23.54 MB | 20.18 MB | 94 | |

The build reports two assets no source names, kept in the start-up bundle: `/Assets/CurvedHeader/avatar.jpg` and `/Pages/teapot.bin`.

First frame, `first-frame.mjs --encoding gzip --phases --runs 3`, medians of three loads, two interleaved rounds; before is a site built from `main` at `fbb9418`, after is this change; both with the pinned toolchain (Rust 1.90.0, Emscripten 6.0.10, `wasm-bindgen` 0.2.129), headless Chromium with WebGL on SwiftShader, 4-core container, nothing else running:

| | Before | After |
|---|---:|---:|
| unthrottled: start-up bundle downloaded | 197, 206 ms | 49, 55 ms |
| unthrottled: first frame | 2,307, 2,426 ms | 2,271, 2,324 ms |
| 50 Mbit/s, 40 ms: module downloaded | 3,926, 3,905 ms | 2,170, 2,167 ms |
| 50 Mbit/s, 40 ms: `runMain` start | 5,441, 5,415 ms | 2,235, 2,237 ms |
| 50 Mbit/s, 40 ms: first frame | **7,393, 7,335 ms** | **4,121, 4,184 ms** |

Unthrottled, the start-up was bound by the CPU and the bundle arrived before the module, so nothing changes there. On the throttled network the module no longer shares the bandwidth with 20.2 MB of assets, and the application starts as soon as the module is instantiated: the first frame comes 3.2 s earlier.

## 20. What the catalog module is made of after E4, and the plan to shrink it (2026-10-06)

The published catalog module, with the themes loaded from compiled markup (stage E4 of the XAML compiler), is 43.84 MB raw and 11.30 MB with gzip -9 (GitHub Pages sends 11.59 MB: its gzip level is lower). This section attributes it, measures what a browser does with it at start-up, and ranks what would make it smaller and cheaper to compile, mobile first. Only the tools changed with it (see "Tools"); nothing in the module changed.

How it was measured. The published module has no name section, and the fat-LTO link of the catalog needs about 15.5 GB of memory (`xaml-compiler/HANDOVER.md`, section 9), more than the 15 GB machine of this pass has without swap. The attribution therefore uses a second link of the same sources (`main` at `666587c`) with thin LTO in one code generation unit, the names kept and a link map: `CARGO_PROFILE_BROWSER_LTO=thin cargo rustc --profile browser --target wasm32-unknown-emscripten -p control-catalog-browser --bin control-catalog-browser -- -C link-arg=--profiling-funcs -C link-arg=-Wl,--Map=<file>`. Its code section is 36.92 MB against 36.36 MB of the published module and its data section 7.22 MB against 7.06 MB, so the shares below carry over within about 2 %. Attribution with `scripts/browser/wasm-size-report.py --map <file> --marginal --functions 50`. Compile times with Node 22.22 (V8 12.4) on the published module, one fresh process per measurement (`--no-wasm-native-module-cache-enabled`); start-up with `scripts/browser/first-frame.mjs` in headless Chromium 141 (SwiftShader WebGL2) on a 4-core container that other builds shared (load average 3 to 5): the absolute times are of this machine, not of a phone, and only their proportions are used. Sizes in MB of 1,000,000 bytes, gzip -9 unless stated. **[M]** marks a measurement, **[E]** an estimate with the reasoning next to it.

### Composition

| Origin (code section, 36.92 MB of the named link) | MB | MB gzip (alone) | MB gzip saved without it | Ran before the first frame |
|---|---:|---:|---:|---:|
| Rust `ferroui_base` | 8.78 | 1.70 | 1.71 | 18 % |
| Rust `ferroui_themes_fluent` (generated `compiled_xaml.rs`) | 7.55 | 1.14 | 1.14 | 38 % |
| Rust `ferroui_themes_simple` (generated `compiled_xaml.rs`) | 4.25 | 0.69 | 0.69 | 7 % |
| C++ Skia with FreeType (`libskia_bindings` of the link map) | 3.38 | 1.27 | 1.29 | 33 % |
| Rust `core` (closures, iterators, formatting) | 2.96 | 0.82 | 0.83 | 19 % |
| functions without an entry in the name section | 2.18 | 0.49 | 0.51 | 28 % |
| Rust `ferroui_controls` | 1.68 | 0.48 | 0.49 | 22 % |
| Rust `ferroui_markup_xaml_loader` (run-time loader) | 1.57 | 0.39 | 0.39 | 87 % |
| Rust `control_catalog` | 0.70 | 0.16 | 0.17 | 9 % |
| drop glue of all types (`drop_in_place<_>` of every crate; not in the crate rows) | 0.61 | 0.10 | | |
| C/C++ not attributed by the map | 0.47 | 0.17 | 0.18 | 20 % |
| Rust `xamlx` | 0.40 | 0.12 | 0.12 | 90 % |
| Rust `ferroui_markup_xaml` (runtime library, `rt` of generated code) | 0.34 | 0.08 | 0.09 | 64 % |
| Rust `alloc`, `std`, `hashbrown` | 0.67 | 0.18 | 0.20 | |
| C++ HarfBuzz | 0.31 | 0.11 | 0.12 | 32 % |
| C image codecs and zlib (inside Skia's archive) | 0.31 | 0.09 | 0.09 | 9 % |
| C/C++ runtime (libc, libc++, Emscripten) | 0.20 | 0.07 | 0.07 | |
| Rust `ferroui_browser`, `ferroui_skia`, `skia_safe` | 0.30 | 0.10 | | |
| Rust `roxmltree` | 0.05 | 0.02 | 0.02 | |
| other Rust crates (`rust_decimal`, `time`, `unicode_normalization`, `tinyvec`, `wasm_bindgen`, ...) | 0.01 | | | |
| wasm-bindgen glue in the module (externref shims) | < 0.01 | | | |

| Other sections (published module) | MB | MB gzip |
|---|---:|---:|
| data | 7.06 | 2.13 |
| of which the six Inter fonts (`ferroui-fonts-inter`) | 1.88 | 0.92 (file by file) |
| of which the markup documents of the catalog (still loaded at run time) | 1.19 | 0.25 |
| of which the markup documents of the Fluent and Simple themes (compiled, but still registered as assets) | 0.94 | 0.16 |
| of which Roboto Light (`ferroui-dialogs`) | 0.17 | 0.09 |
| of which other constant data: about 1.0 MB of text (type names 0.45, markup names 0.22, panic locations 0.07) and 2.0 MB of tables and vtables | 2.98 | |
| element (function table) | 0.30 | 0.24 |
| type, function, import | 0.12 | 0.04 |
| script of the module (`control-catalog-browser.js`, not in the module) | 0.22 | 0.05 |

**[M]** all rows. "Alone" compresses the bytes of a row by themselves; "saved without it" is the gzip of the whole code section minus the gzip of the code section without the row, which is what removing the row saves in transfer; the two agree within 0.02 MB for every row, so code of one origin compresses against itself, not against the rest. "Ran before the first frame" is the share of the row's bytes in functions that ran up to the first frame (instrumented module, see "Start-up"). The functions without a name entry are spread over all origins (neighbours of named Rust, Skia and HarfBuzz functions); their origin is unknown. Third-party Rust crates are 0.20 MB together (`hashbrown` 0.10, `roxmltree` 0.05, `skia_safe` 0.04, the rest 0.01), no code of Skia's PDF or SVG back ends is linked (the link map has no `SkPDF` symbol) and no ICU data is in the module, so the default features of `skia-safe` (`pdf`, `jpeg`, `embed-icudtl`, `binary-cache`) cost nothing; the dependency tree of the browser build (`cargo tree -e features`) has no `regex`, `serde`, `image` or similar crate.

The largest functions are all generated: 47 of the 50 largest (6.2 MB together) are build functions of `compiled_xaml.rs`, led by `populate_fluenttheme_xaml` (1.38 MB in one function, the resource dictionary of the Fluent theme) and the closures of the control themes of `build_controls_fluentcontrols_xaml` and `build_controls_simplecontrols_xaml` (60 to 285 kB each). The three others are `RuntimeTypeSystem::...` of the loader (88 kB), Skia's `skcms_private::baseline::exec_stages` (85 kB) and libjpeg's `encode_mcu_huff` (59 kB).

Largest generic families outside the generated code (instances; MB raw; MB gzip alone): `FnOnce::call_once` shims (18,000; 2.09; 0.62: 0.52 MB of them are the by-name markup invokers that call `MarkupArguments::next`, 0.49 MB class vtable shims), `drop_in_place<_>` (5,559; 0.61; 0.10), `MarkupArguments::next<T>` (437; 0.55; 0.05), `FnOnce::call_once{{vtable.shim}}` (9,095; 0.37; 0.07), `StyledProperty<T>::from_untyped` (220; 0.24; 0.03), typed observers `on_next` (1,570; 0.22; 0.03), `TypedBindingExpression<T>::write_source_value_to_target` (234; 0.22; 0.02), `Interactive::add_handler_as` closures (274; 0.20; 0.05), `coerce_value` (220; 0.17; 0.03), `BindingEntry<T>::set_value` (220; 0.15; 0.01), the value type registry closures (`register_conversion`, `register_cast`, `register_object`, `register_nullable`: about 0.47 MB over 5,000 instances). The generic helpers that generated code calls (`rt::cast`, `rt::to_object`, `rt::exact`, `rt::bind`, `rt::to_value`, `rt::deferred_builder`) are 0.11 MB in 2,583 instances: they are small, and the cost of generated code is at its call sites (next paragraph). **[M]**

### Where the bytes of generated code go: exceptions

Rust on this target unwinds with JavaScript exceptions (section 18). The linker rewrites every call that may unwind into a call of an imported `invoke_*` function, with the callee passed as a table index, and brackets it with stores and a load of the `__THREW__` flag in memory and a branch to the landing pad. The module makes 801,840 calls (29,486 of them indirect), and 328,493 of them are such `invoke_*` calls, with 110,917 landing pads (`__cxa_find_matching_catch_2`). In the generated theme code it is 167,892 of 214,064 calls: generated build functions hold many reference-counted locals, so nearly every call needs a cleanup path. **[M]** (instruction scan of the named module.)

What that costs:

- Size: the two `__THREW__ = 0` stores per call site (658,525), with the few loads and checks of the flag that a byte-pattern scan recognises, are 6.82 MB of the 36.92 MB code section (30.10 MB without them), but only 0.30 MB of its gzip (8.36 to 8.06 MB): the sequence is the same bytes everywhere. **[M]** With the load, the comparison and the table index the bookkeeping is about 41 bytes per call site (read from the disassembly of a generated function: 58 `invoke_*` calls, 174 accesses to the flag), so 10 to 13 MB raw in all. **[E]**
- Start-up: every such call leaves WebAssembly for the script of the module and comes back through a table lookup. In CPU profiles of three loads up to the first frame (named module, means), the wasm-to-JavaScript and JavaScript-to-wasm transitions took 0.80 s, `getWasmTableEntry` 0.13 s and `invoke_*` with `stackSave` 0.05 s: about 1.0 s of the 2.85 to 3.20 s to the first frame, against 0.25 s in `ferroui_base` itself, 0.24 s in the run-time loader with `xamlx` and the runtime library, 0.09 s in Skia, and 0.26 s waiting for SwiftShader to compile shaders. **[M]** (The table lookup cache of section 18 is in place; without it the share was larger.)

### Start-up: what runs, and what compiling costs

Up to the first frame of the home page 19,573 of the 113,497 functions of the published module run, 10.17 MB of its 36.36 MB of code (28 %). **[M]** (`wasm-split --instrument` of binaryen on the published module, profile read after the first frame with `first-frame.mjs --wasm-profile`.) By origin (named module): 87 % of the loader and 90 % of `xamlx` run, the catalog loads its pages through them; 38 % of the Fluent theme code (the resource dictionary and the control themes the first page shows); 7 % of the Simple theme, which the catalog creates as upstream does but whose control themes stay deferred.

Compiling the published module in V8 (Node 22.22, V8 12.4; this container): **[M]**

| V8 configuration | Time |
|---|---:|
| Default: validation of the whole module, functions compiled lazily on their first call (Liftoff), hot ones later with TurboFan in the background | 0.13 to 0.23 s over two series (then each function on its first call) |
| Lazy validation as well (`--wasm-lazy-validation`) | 0.05 to 0.11 s |
| Every function with Liftoff up front, 4 threads (`--no-wasm-lazy-compilation`) | 0.94 to 1.10 s over two series |
| The same on one thread | 1.58 to 1.76 s |
| Every function with TurboFan up front, 4 threads | 15.6 to 22.2 s |

What that means per engine:

- **Chrome** compiles lazily: start-up pays the validation of the whole module plus Liftoff for each function the first time it runs, on the main thread. Liftoff of the 10.2 MB that run is about 0.45 s on one thread of this machine **[E]** (from the 1.6 s for 36.4 MB), several times that on a phone. TurboFan compiles hot functions in the background (dynamic tiering). **Code cache:** for a module of 128 kB or more compiled with `instantiateStreaming` (the script of the module does), Chrome stores the TurboFan code next to the HTTP cache entry once enough of it exists, and reuses it while the cached response stays valid; a `304 Not Modified` keeps it, a `200` replaces the resource and drops it, and a changed URL (also a query string) is a new entry. Liftoff code is not cached. **[V]** (v8.dev, "Code caching for WebAssembly developers"; "WebAssembly compilation pipeline".)
- **Firefox** compiles every function with its baseline compiler while the module streams in, then with Ion in the background (tiered compilation), so start-up pays for all 36 MB of code, not only for what runs; caching of stream-compiled modules in the HTTP cache landed in Firefox 67 behind `javascript.options.wasm_caching` (bug 1487113). Whether it is on in current releases was not checked. **[V]** / **[U]**
- **Safari** (JavaScriptCore) starts functions in its in-place interpreter (IPInt) and tiers hot ones up to BBQ and OMG, so it compiles little at start-up but runs the start-up code interpreted at first. **[V]** (WebKit blog, JetStream 3.) No persistent WebAssembly code cache is documented. **[U]**
- **GitHub Pages** sends `cache-control: max-age=600`, `last-modified` and a weak `etag` made of the modification time and the size of the file (`W/"6ac5059b-29cff09"` for the module on 2026-10-06), and gzip only (no Brotli, also when the browser offers it). **[M]** (`curl -I`.) So a repeat visit within ten minutes uses the cached module without a request; after that it is revalidated, and a `304` keeps Chrome's code cache. Every deployment of the site gives every file a new modification time, hence a new ETag and a full download and recompile, even when the module did not change; the module's URL never changes, which is what the code cache needs. A content hash in the file name would not help on Pages (it cannot send `immutable`), so nothing is to be changed here.

### Split module (`-sSPLIT_MODULE`, binaryen `wasm-split`): verdict

Measured on the published module with the start-up profile above (`wasm-split --split --profile`): **[M]**

| | MB | MB gzip | Compile (V8, lazy / eager Liftoff, 4 threads) |
|---|---:|---:|---:|
| Published module | 43.84 | 11.30 | 0.13 to 0.15 s / 0.94 to 1.00 s (same series as the next row) |
| Primary module (the functions that ran up to the first frame, all data) | 21.69 | 5.79 | 0.08 to 0.09 s / 0.37 to 0.42 s |
| Secondary module (the rest) | 27.07 | 6.57 | |
| Both | 48.76 | 12.36 | |

The split moves 5.5 MB of gzip off the download before the first frame and 0.6 s of eager baseline compilation off a Firefox-like start-up, and costs 4.9 MB raw and 1.06 MB gzip in total (the exports and imports between the two halves and the placeholder table entries). It is not worth doing with this build now:

- **Loading the secondary module on the main thread is not possible synchronously.** The runtime of Emscripten 6.0.10 (`src/preamble.js`) replaces each moved function by a placeholder that calls `loadSplitModule`, by default `instantiateSync`: a synchronous read of the file (there is none on the web main thread: `readBinary` exists only in workers) and `new WebAssembly.Module` / `new WebAssembly.Instance`, which Chrome refuses on the main thread for modules over 8 MB (the secondary is 27 MB). The alternative of the runtime is JSPI (`-sJSPI`), where the placeholder suspends until the file is there; JSPI ships in Chrome 137, Firefox 153 (July 2026) and Safari 27 (September 2026), so older iOS and Android browsers are excluded. But under JSPI every export through which a moved function can be reached must be a promising export, which returns a promise: the boundary of this backend answers synchronously (an input handler says whether the event was handled, section 5), so that is a redesign of the boundary, not a build option. **[V]** (Emscripten source, `settings.js`; MDN browser compat data; Firefox 153 release notes.)
- **Without JSPI** the host page would have to fetch and instantiate the secondary module asynchronously right after the primary one and keep the application from reaching any moved function until then. The profile covers one page in one rendering mode at one size; a function it missed (another page, Software2D, a resize, the first key press) would trap instead of running. That is a behaviour change, not an optimisation.
- **The function table lookup cache** of section 18 (`wasm_table_mirror.js`) assumes that the table is only written through `setWasmTableEntry`; the secondary module writes its functions into the table with its element segment, so the cache would keep calling the placeholders. It would need invalidation on load.
- **The build** would need an instrumented link, a profiling run in CI and a second `wasm-split` pass, on top of a fat-LTO link that already needs 15.5 GB.
- **The first frame waits for the asset bundle anyway**: the host page fetches `control-catalog.assets` (23.54 MB, 20.18 MB gzip) and calls `runMain` only when both the module and the bundle are there (`wwwroot/main.js`). Halving the module's download moves the first frame only once the bundle is off that path (item 6 below).

What a prototype would need: the `browser` link with `-sSPLIT_MODULE` (writes the instrumented module and `.orig`); a run with `first-frame.mjs --wasm-profile` over several pages and both rendering modes; `wasm-split --split --profile=<merged profiles> <module>.orig`; a host page that instantiates `<module>.deferred.wasm` asynchronously with `{primary: exports}` and provides `Module.loadSplitModule`; invalidation of the table cache; and a gate on input until the secondary module is in. Revisit if a later JSPI-compatible design of the boundary exists, or if the application's own lazy loading (pages) makes a natural split point.

### The plan, ranked

Ranked by what it does for a phone: bytes to compile and execute at start-up first, then transfer. "Upstream" says whether the behaviour stays as upstream's.

| # | Change | Raw | Gzip | Compile and start-up | Risk | Upstream |
|---|---|---:|---:|---|---|---|
| 1 | **WebAssembly exception handling** instead of JavaScript exceptions: Rust `-Z emscripten-wasm-eh`, Emscripten `-fwasm-exceptions` (legacy instructions, the default of 6.0.10: Chrome 95, Firefox 100, Safari 15.2), the standard library rebuilt for it | -10 to -13 MB **[E]** (lower bound -6.8 MB **[M]**) | -0.3 to -0.6 MB **[E]** | Removes the JavaScript round trip of 328,493 call sites: about 1.0 s of 2.9 s to the first frame here **[M]**; every engine compiles a quarter to a third less code | Toolchain: the flag is unstable on the pinned 1.90.0 and on stable 1.97.0 (`rustc -Z help`), and the target's prebuilt standard library uses JavaScript exceptions, so it needs a nightly with `-Z build-std` or a stable release that adopts it; the link with Skia's prebuilt archive needs checking; `wasm_table_mirror.js` becomes unnecessary | Same unwinding semantics (`catch_unwind`, re-raised panics); a build change only |
| 2 | **Shared `rt` helpers in generated code**: one call per setter (`Setter` with property and value added to its style: 2,890 in the two themes), per template binding (1,933), per markup extension value with its target property bookkeeping (1,675), per deferred content preamble (2,123); `#[inline(never)]` where LLVM still inlines | -2.5 to -3 MB **[E]** (about 55 bytes per call removed, 3 to 8 calls per pattern) | -0.4 MB **[E]** | 3.2 MB of theme code runs before the first frame **[M]**; proportionally less to compile; splitting `populate_fluenttheme_xaml` (1.38 MB, one function) shortens the single largest lazy compilation on the main thread | Emitter change with regenerated `compiled_xaml.rs`, drift tests and the differential harness; overlaps item 1 (fewer and cheaper call sites) | Same calls in the same order, inside the helper |
| 3 | **Stage E5 for the catalog**: its 219 documents compiled, `x:Class` documents of the dialogs compiled, so neither links the run-time loader (upstream's catalog does not use `AvaloniaRuntimeXamlLoader`) | Removes the loader, `xamlx`, `roxmltree` and their drop glue (2.1 MB **[M]**) and the 1.19 MB of catalog documents; adds the catalog's generated code, about 13 MB at today's ratio of the themes (11.8 MB of code from 1.15 MB of markup) or about 9 MB after item 2: **net +6 to +10 MB** **[E]** | -0.8 MB removed, +1.3 to +2.0 MB added: **net +0.5 to +1.2 MB** **[E]** | Removes the run-time parse and transform: 0.24 s of wasm CPU to the first frame here plus their share of item 1's transitions **[M]**; on the desktop the themes' load went from 0.5 s to 2.5 ms with E4 | Large (stage E5); the module grows unless item 2 lands first | Upstream compiles the catalog |
| 4 | **Compiled documents out of the embedded assets**, as upstream's compiler removes every compiled resource from the assembly (`res.Remove()` in `XamlCompilerTaskExecutor`) and answers loads by URI through the generated `!XamlLoader` | -0.94 MB now (themes), -2.13 MB with item 3 **[M]** | -0.16 / -0.41 MB **[M]** | None (data is not compiled) | Needs E5's loader table first: today an application can still `StyleInclude` a theme document by URI, which the run-time loader serves from these assets | Yes, once the loader table exists |
| 5 | **Generic families**: outline the type-independent parts of `MarkupArguments::next<T>` (437 instances, 0.55 MB), the markup invoker shims (0.52 MB), `TypedBindingExpression<T>` and the observers, as the desktop pass did; the property store needs the type-erased design of `desktop-performance.md` item 10 | -0.5 to -1 MB **[E]** | < 0.1 MB **[E]** | Little: 18 % of `ferroui_base` runs at start-up | Hot paths; each split with the property system tests | Same statements, same order |
| 6 | **The asset bundle on the start-up path** (not the module): the page waits for 20.18 MB of gzip of pictures and fonts (64 % of the site's transfer) before `runMain`. Per-page bundles fetched before a page is shown would take most of it off the first frame | 0 for the module | up to -20 MB before the first frame **[E]** (what the first page needs is not measured) | The largest single wait on a phone network | Asset access is synchronous (upstream `AssetLoader.Open`), so loading on demand needs a page-level await in the catalog, not in the framework; the catalog belongs to its worker | Sample-specific; the framework stays as upstream |
| 7 | Split module | -5.5 MB gzip before the first frame, +1.06 MB in total | | See the verdict above | High | Not without a redesign of the boundary |
| 8 | Inter fonts to the asset bundle | -1.88 MB module | -0.92 MB module, 0 for the site | None | Low | `Avalonia.Fonts.Inter` embeds them; only useful for the module's gzip budget |

Measured and not worth doing:

- **Post-link optimisation is converged.** `wasm-opt -Oz` once more on the published module: 43.80 MB (-0.04) and the same gzip; `--merge-similar-functions` alone makes it 0.06 MB larger. **[M]**
- **Dependencies and features**: nothing unused is linked (see "Composition"); Skia's feature set is fixed by its prebuilt binaries (section 3).
- **Generic `rt` helpers** of generated code: 0.11 MB for all their instances; outlining their error paths would save a few kB.
- **Caching headers**: nothing to change on GitHub Pages (see "Start-up").

Next steps in order: item 1 needs a toolchain decision by the owner (nightly with `build-std` for the browser build only, or waiting for a stable release) and then one measured build; item 2 is the next emitter task and should land before item 3; item 4 comes with E5's loader table; item 6 is the catalog's.

### Tools

- `scripts/browser/wasm-size-report.py`: `--map <link map>` attributes C and C++ functions by the archive the link map of wasm-ld names (Skia, HarfBuzz and the runtime exactly); `--marginal` adds the gzip saved without each row; `--functions <n>` lists the largest functions; `--profile <file>` shows how much of each origin ran in a profiled run; demangled C++ names with a return type are attributed, functions without a name entry and the externref shims of wasm-bindgen get rows of their own.
- `scripts/browser/first-frame.mjs --wasm-profile <file>`: for a module instrumented with `wasm-split --instrument`, writes the profile of the functions that ran up to the first frame of the last load, the input of `wasm-split --split --profile` and of `wasm-size-report.py --profile`.

Commands of this section:

```
source .tools/env.sh
# named link with a link map (thin LTO: the fat-LTO link of the catalog needs about 15.5 GB)
CARGO_PROFILE_BROWSER_LTO=thin cargo rustc --locked --profile browser --target wasm32-unknown-emscripten \
    -p control-catalog-browser --bin control-catalog-browser -- \
    -C link-arg=--profiling-funcs -C link-arg=-Wl,--Map=catalog.map
python3 scripts/browser/wasm-size-report.py <module.wasm> --map catalog.map --marginal --functions 50 --top 40
python3 scripts/browser/wasm-size-report.py <published module> --assets samples src     # embedded files
# start-up profile: instrument, serve in place of the module, run to the first frame
FEATURES="--enable-bulk-memory --enable-bulk-memory-opt --enable-sign-ext --enable-mutable-globals \
    --enable-nontrapping-float-to-int --enable-reference-types --enable-multivalue --enable-call-indirect-overlong --enable-threads"
wasm-split --instrument $FEATURES <module.wasm> -o <site>/control_catalog_browser.wasm
node scripts/browser/first-frame.mjs <site> --runs 1 --wasm-profile startup.prof
python3 scripts/browser/wasm-size-report.py <module.wasm> --profile startup.prof
wasm-split --split $FEATURES --profile=startup.prof <module.wasm> -o1 primary.wasm -o2 secondary.wasm
# compile time of one configuration (fresh process each time)
node --no-wasm-native-module-cache-enabled [--no-wasm-lazy-compilation] [--wasm-num-compilation-tasks=1] compile.mjs <module.wasm>
```

`compile.mjs` is three lines: read the file, `await WebAssembly.compile(bytes)`, print the time. The instrumented and split modules need the feature flags above (the module uses atomics from Skia's archive and the bulk memory and reference types of Emscripten 6.0.10).

Sources (checked 2026-10-06): V8, "WebAssembly compilation pipeline" (https://v8.dev/docs/wasm-compilation-pipeline) and "Code caching for WebAssembly developers" (https://v8.dev/blog/wasm-code-caching); Mozilla bug 1487113 (alt-data caching of stream-compiled modules); WebKit, "Introducing the JetStream 3 Benchmark Suite" (IPInt, BBQ, OMG); MDN browser-compat-data pull request 30552 (JSPI in Safari 27) and the Firefox 153 release notes (JSPI); Chromium's 8 MB limit on synchronous compilation on the main thread (`v8_initializer.cc`); Emscripten 6.0.10 `src/settings.js` (`SPLIT_MODULE`, `WASM_LEGACY_EXCEPTIONS`), `src/preamble.js` (`splitModuleProxyHandler`, `instantiateSync`), `tools/link.py` (`do_split_module`); upstream `src/Avalonia.Build.Tasks/XamlCompilerTaskExecutor.cs` (`res.Remove()`).

## Owner decision (2026-10-04)

Proceed as recommended: `wasm32-unknown-emscripten` with the Skia backend first (Ganesh/WebGL2 and raster paths kept in `FerroUI.Skia` next to Graphite/Metal), Vello on `wasm32-unknown-unknown` second.
