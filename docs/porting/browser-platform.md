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
  Cargo.toml, main.rs        binary crate (required by wasm-bindgen on Emscripten)
  wwwroot/index.html, app.css, main.js
```

Workspace changes: the crates above as members, plus `src/FerroUI.OpenGL` (the OpenGL contracts, upstream `Avalonia.OpenGL`, which the Ganesh path of `FerroUI.Skia` and the WebGL render target implement); `skia-safe` features per target in the manifest of `FerroUI.Skia` (`graphite`+`metal` on Apple targets, `gl` on Emscripten); `FerroUI.Native` and `FerroUI.Desktop` are never built for the WASM target (the browser build selects its package).

Tooling:

- **Emscripten configuration**: plain `cargo build --target wasm32-unknown-emscripten` plus a repository script (`scripts/build-browser.sh` or an `xtask`) that runs esbuild, builds, and assembles `wwwroot` + `.js` + `.wasm` into a `dist` directory served by any static server. No trunk and no wasm-pack (wasm-pack has an open request for this target; trunk support **[U]**).
- Link settings (in `.cargo/config.toml` for the target) **[M]**: `linker = "em++"` (Skia needs the C++ runtime), `-sWASM_BINDGEN`, `-sMODULARIZE`, `-sEXPORT_ES6`, `-sENVIRONMENT=web`, `-sMAX_WEBGL_VERSION=2`, `-sALLOW_MEMORY_GROWTH=1`, `-sEXPORTED_RUNTIME_METHODS=GL,HEAPU8` (the JS side must reach Emscripten's `GL` object and the module memory), `-sINVOKE_RUN=0` (the host page starts the application; without it `main` also runs twice), `-sSTACK_SIZE=8MB` (the default stack of 64 KB is too small for layout and markup loading), `-sGL_ENABLE_GET_PROC_ADDRESS=1` (the OpenGL entry points of a context are resolved by name), and `EMCC_CFLAGS="-s ERROR_ON_UNDEFINED_SYMBOLS=0"` as rust-skia requires. No `-Cpanic=abort` and no `-Crelocation-model=static`. The backend imports its script module by a relative specifier, so there are no `wasm-bindgen` snippets to copy: `scripts/build-browser.sh` assembles the site from the host page, `ferroui.js` and the `.js`/`.wasm` pair of the application (`target/wasm32-unknown-emscripten/<profile>/examples/` for an example of the browser crate). It builds the cargo profile `browser` (size-optimised; links at `-O2`, then runs `wasm-opt -Oz` and writes `.gz`/`.br` copies), see `browser-size.md`.
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
- Size of that example, release profile of the workspace, not optimised for size: 48.7 MB of WASM, 10.2 MB with gzip. After the first size pass (`browser-size.md`, the `browser` profile): 27.1 MB, 7.8 MB with gzip, 5.1 MB with brotli; where the bytes are and what remains is in that document.

### Still not verified

- Firefox, Safari and mobile browsers; WebGL1; the `failIfMajorPerformanceCaveat` fallback to the raster path.
- Whether all of `web-sys`, `js-sys` and `wasm-bindgen-futures` are usable on the Emscripten target (the boundary rules of section 5 keep them out of the port).
- Whether a newer Rust toolchain or a rebuilt standard library allows `-Cpanic=abort` on the target, and what it would save.
- Size and start-up of the full framework in the browser, and the speed of the run-time XAML loader there.
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
| `input.ts` key handlers | Handlers return a promise and call `preventDefault` in `.then`; `keydown` prevents default unless the event was handled while a clipboard read is pending, `keyup` prevents default when **not** handled. In effect almost every key's default action is suppressed while the host has focus | Return `bool` synchronously; define an explicit policy for which browser shortcuts pass through |
| `input.ts` pointer unsubscription | Removes `pointerover` instead of `pointermove`; the `beforeinput` listener is never removed | Fix |
| `BrowserInputHandler.OnWheel` | Fixed divisor 50, `deltaMode` ignored | Honour `deltaMode` |
| `softwareRenderTarget.ts` | A premultiplied framebuffer is passed to `putImageData`, which expects straight alpha; translucent pixels over page content come out wrong | Fixed: the frame is converted to straight alpha into a retained buffer before it is put on the canvas (opaque and transparent pixels are copied unchanged); `webapp/tests/software-blit.test.mjs` checks the composited pixels in headless Chrome |
| `BrowserTopLevelImpl` | `PointToScreen` is the identity; `LostFocus` is never raised; `Dispose` does not unsubscribe input | Fix in Phase 2/3 |
| `BrowserInputHandler.OnPointerMove` | The coalesced-points loop steps the index by the item size while bounding by point count | Re-derive rather than transliterate |
| `stream.ts` `write` | The copy fallback writes the original span, not the copy | Rewritten anyway |
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
| `Program.cs` | Entry: build app, parse options from the query string, start on `"out"` | `main.rs` | 1 (minimal), 3 (catalog) |
| `ControlCatalog.Browser.csproj` | WASM SDK project, threads off | `Cargo.toml` | 1 |
| `wwwroot/index.html`, `app.css`, `favicon.ico` | Host page with splash | same | 1 |
| `wwwroot/main.js` | Runtime bootstrap | `main.js` importing the generated module | 1 |
| `EmbedSample.Browser.cs`, `wwwroot/embed.js` | Native control host demo (iframe, DOM button) | `embed_sample_browser.rs`, `embed.js` | 4 |

## 16. Sources

- rust-skia README (Emscripten support, prebuilt targets): https://github.com/rust-skia/rust-skia
- skia-safe on crates.io (0.153.3, 2026-09-04): https://crates.io/crates/skia-safe
- harfbuzz-sys on crates.io (0.8.0) and build script: https://crates.io/crates/harfbuzz-sys, https://github.com/servo/rust-harfbuzz
- wasm-bindgen guide, Emscripten target: https://wasm-bindgen.github.io/wasm-bindgen/reference/emscripten.html
- wgpu Emscripten issue and PR: https://github.com/gfx-rs/wgpu/issues/10274, https://github.com/gfx-rs/wgpu/pull/10515
- Vello README: https://github.com/linebender/vello
- HarfRust releases; rustybuzz archival: https://github.com/harfbuzz/harfrust/releases, https://github.com/harfbuzz/rustybuzz
- Local Font Access API: https://developer.mozilla.org/en-US/docs/Web/API/Local_Font_Access_API
- WebGPU overview: https://developer.chrome.com/docs/web-platform/webgpu/overview

## Owner decision (2026-10-04)

Proceed as recommended: `wasm32-unknown-emscripten` with the Skia backend first (Ganesh/WebGL2 and raster paths kept in `FerroUI.Skia` next to Graphite/Metal), Vello on `wasm32-unknown-unknown` second.
