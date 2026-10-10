# Browser: Skia Graphite on WebGPU (Dawn's Emscripten bindings)

Status: investigation only, 2026-10-09. Nothing was built, run or downloaded for it. No decision is taken here.

Question: can the browser build render with Skia Graphite over WebGPU, instead of or beside today's Skia Ganesh on WebGL2, and what would it take?

Marks used below:

- **[R]** read in a source file on this machine (path and line given).
- **[G]** general knowledge, not confirmed in any local source.
- **[U]** unknown: could not be settled without building or downloading.

Paths that start with `skia-bindings/`, `skia-safe/` are in `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/skia-bindings-0.153.3/` and `.../skia-safe-0.153.3/` (the versions `Cargo.lock` pins, lines 806 to 826). Paths that start with `skia/` are the Skia source tree at `skia-bindings-0.153.3/skia/` (see section 2 for why it is there). Paths that start with `emscripten/` are in `.tools/emsdk/upstream/emscripten/`.

## Summary

**Feasible in principle, not available today, and not a small step.** Every layer exists somewhere, but no two adjacent layers fit together as they are:

1. Skia has a Graphite backend for Dawn/WebGPU that compiles for Emscripten and can run without Asyncify ("non-yielding" context). **[R]**
2. The Rust bindings wrap Graphite for Metal and Vulkan only. There is no Dawn feature, no `gn` argument for it, no shim function and no Rust module. **[R]**
3. The Emscripten branches of Skia's Dawn backend at the pinned Skia tag are written against Emscripten's old built-in WebGPU bindings (`-sUSE_WEBGPU`). The pinned Emscripten 6.0.10 removed those; only the `emdawnwebgpu` port remains, which has the newer `webgpu.h`. About a dozen places in eight Skia files have to be ported (or a newer Skia taken, if it has done so: **[U]**). **[R]**
4. Skia's `gn` rules for `skia_use_dawn` build Dawn natively with CMake from `third_party/externals/dawn`, which the Skia fork of the bindings does not even list in `DEPS`. For the browser no native Dawn is wanted at all; the rule has to be bypassed. **[R]**
5. No published binary of the bindings carries Graphite for this target, so Skia has to be built from source for WebAssembly, twice if both kinds of module (with and without threads) are kept. **[R]**
6. On FerroUI's side most of the Graphite work is already backend-neutral (`GraphiteGrContext`); what is new is a GPU and render target for Dawn, the script side that requests the device and hands it over, a rendering mode, loss handling and an asynchronous readback. **[R]**

The work is therefore mostly below FerroUI: a fork of the bindings (both crates), a patch set on Skia, and a build of Skia from source in the tools set-up and in CI. The first experiment (section 6, stage 1) is designed to answer the three unknowns that can stop the whole thing before any FerroUI code is written.

Upstream's browser backend has WebGL and software only (section 4), so this would be an addition beyond upstream and has to be recorded as one in `DEVIATIONS.md`.

## 1. The Rust bindings

### What the locked versions expose

- Features of `skia-safe` 0.153.3 (`skia-safe/Cargo.toml`, `[features]`): `ganesh`, `gl`, `egl`, `x11`, `wayland`, `vulkan`, `metal`, `d3d`, `graphite`, and the non-GPU ones. `graphite = ["skia-bindings/graphite"]`. **There is no `dawn` or `webgpu` feature.** The same holds for `skia-bindings/Cargo.toml` (`[features]`: `d3d`, `egl`, `ganesh`, `gl`, `graphite`, `metal`, `vulkan`, ...). **[R]**
- The read-me of the crate lists WebGPU/Dawn as an unchecked item (`skia-safe/README.md:30`, under Ganesh, where Skia has since removed it), and Graphite as a whole as unchecked (line 31). **[R]**
- Modules of `skia-safe/src/gpu/graphite/`: `backend_texture`, `context`, `context_options`, `graphite_types`, `image`, `recorder`, `recording`, `surface`, `texture_info`, and the two backends `mtl` (`#[cfg(feature = "metal")]`) and `vk` (`#[cfg(feature = "vulkan")]`) (`skia-safe/src/gpu/graphite.rs:34-76`). A search for `dawn`, `webgpu` and `wgpu` over the sources of both crates finds only the read-me line. **[R]**
- The shim `skia-bindings/src/graphite.cpp` (434 lines) includes the Metal headers under `SK_METAL` (lines 24-27) and the Vulkan ones under `SK_VULKAN` (29-32), and ends with `C_MtlBackendContext_Construct`, `C_ContextFactory_MakeMetal`, `C_BackendTextures_MakeMetal` (376-413) and `C_ContextFactory_MakeVulkan` (415-434). Nothing for `SK_DAWN`. **[R]**

### What `gn` arguments each feature sets

`skia-bindings/build_support/skia/config.rs`, `FinalBuildConfiguration::from_build_configuration`:

| Feature | Argument | Line |
|---|---|---|
| `ganesh` | `skia_enable_ganesh` | 127 |
| `graphite` | `skia_enable_graphite` | 128 |
| `gl` | `skia_use_gl` | 131 |
| `vulkan` | `skia_use_vulkan=true`, `skia_enable_spirv_validation=false` | 156-160 |
| `metal` | `skia_use_metal=true` | 162-164 |
| `d3d` | `skia_use_direct3d=true` | 166-168 |

`skia_use_dawn` is never set. The platform file for this target, `build_support/platform/emscripten.rs:12-29`, adds `skia_gl_standard="webgl"`, `skia_use_webgl=<ganesh>`, `target_cpu="wasm"`, `skia_emsdk_dir`, and the two font manager arguments. It does not set `is_canvaskit`, so the build is Skia's generic WebAssembly configuration. Extra arguments can be appended from outside through the environment variable `SKIA_GN_ARGS` (`config.rs:329-331`), which is the only way to reach `skia_use_dawn` without changing the crate. **[R]**

### Is Graphite allowed for Emscripten at all

Yes, nothing excludes it. The checks of `skia-bindings/build.rs:20-50` are: `vulkan` or `metal` without an engine is an error (`features.rs:103`), dependent features (`gl` needs `ganesh`, and so on, `features.rs:248-256`), and redundant features of the platform. `emscripten.rs:76-83` only adds `embed-freetype`. `graphite` with neither `metal` nor `vulkan` passes all of them: Skia would be built with `skia_enable_graphite=true` and no backend, which compiles Graphite's core and nothing that can make a context. What stops it in practice is the binary cache: the key of the published archive for this target is `ganesh-gl-jpegd-jpege-pdf` (`.tools/skia-threads/*/complete`), no archive exists for a key with `graphite`, and the crate then starts a build from source (`build.rs:112-151`). `docs/porting/browser-platform.md:35` records that this was tried and failed at dependency sync. **[R]**

### What is missing for a Dawn backend

In `skia-bindings`:

1. A feature (say `dawn`), its constant and key in `build_support/features.rs`, and `backend_without_engine` extended to it (line 103).
2. `config.rs`: `skia_use_dawn=true` for the feature. `emscripten.rs`: whatever replaces Skia's native Dawn rule (section 2), and the compile flag of the port for Skia's sources.
3. `skia_bindgen.rs`: the port's include path and flags for the shim and for the binding generator (`Configuration::new`, lines 46-88, chooses the shim sources; `definitions::ninja_files_for_features`, lines 948-958, already reads `obj/graphite.ninja`, where `SK_DAWN` would appear as a public define, `skia/BUILD.gn:912-915`).
4. Shim functions in `graphite.cpp` under `#ifdef SK_DAWN`, modelled on the Metal ones:
   - `C_DawnBackendContext_Construct(uninitialized, WGPUInstance, WGPUDevice, WGPUQueue)` and `_destruct` (`skia/include/gpu/graphite/dawn/DawnBackendContext.h:61-72`: `fInstance`, `fDevice`, `fQueue`, `fTick`, the last null by default on Emscripten);
   - `C_ContextFactory_MakeDawn` (`DawnBackendContext.h:74-76`);
   - `C_BackendTextures_MakeDawn(uninitialized, WGPUTexture)` (`skia/include/gpu/graphite/dawn/DawnGraphiteTypes.h:131`; the texture reports its own size and format) and, if a view is ever wanted, the overload with `DawnTextureInfo` and `WGPUTextureView` (line 156);
   - optionally `TextureInfos::MakeDawn` (line 116).
   The handles should cross the boundary as opaque pointers, as the Metal shim passes `const void*`, so that the binding generator never sees `webgpu_cpp.h`.
5. Wrapping the canvas's current texture needs no new shim beyond `BackendTextures::MakeDawn`: `C_SkSurfaces_WrapBackendTextureGraphite` (`graphite.cpp:308`) is backend-neutral and is what FerroUI's Metal path already calls through `surfaces::wrap_backend_texture`.

In `skia-safe`: a module `gpu::graphite::dawn` with `BackendContext`, `context_factory::make_dawn`, `backend_textures::make_dawn`, behind the feature. The constructors of the handle types are `pub(crate)` (`skia-safe/src/prelude.rs:477`, `514`: `from_ptr`), so this cannot be added from outside the crate: FerroUI cannot keep the shim in its own build script (as it does for `emscripten_sjlj.cpp`) and turn the raw `Context*` into a `skia_safe::gpu::graphite::Context`. **A fork of both crates, taken through `[patch.crates-io]`, is needed** until the change is accepted by the project. **[R]**

One more gap, independent of the backend: the only readback of the bindings is `C_Context_readPixels` (`graphite.cpp:170-206`), which calls `asyncRescaleAndReadPixels` and then `submit(SyncToCpu::kYes)`. A context without a tick function forbids exactly that (`DawnBackendContext.h:39-48`). An asynchronous readback (callback delivered by `checkAsyncWorkCompletion` on a later turn of the event loop) has to be bound as well. **[R]**

### Newer versions of the crates

Only 0.153.3 is in the local registry (`~/.cargo/registry/cache/*/skia-*.crate`), and the crates carry no change log. Whether a later release adds Dawn: **[U]**.

## 2. Skia

### The tree that was read

`skia-bindings-0.153.3/skia/` is a Skia source tree (192 MB, tag `m153-0.101.2` of the fork `rust-skia/skia`, per `[package.metadata]` of the crate), unpacked there on 2026-10-05 by an earlier fall-back to a build from source (`build_support/binary_cache/download.rs:42-127`). It has no `third_party/externals`. The tree the threads shim downloads is deleted when the script ends (`scripts/browser/skia-threads-shim.sh`, last lines), so `.tools/skia-threads/` holds only the archive, the definitions and the function list. **[R]**

### Build arguments

- `skia/gn/skia.gni:51`: `skia_use_dawn = false`; `:100`: `skia_use_webgpu = is_wasm` (declared and, in this tree, read nowhere else but CanvasKit's `compile.sh`); `:110`: `skia_enable_graphite = false`; `:193`: `assert(!skia_use_dawn || skia_enable_graphite)`, "Dawn is Graphite-only". **[R]**
- `skia/BUILD.gn:912-915`: with `skia_use_dawn`, the public define `SK_DAWN` and a dependency on `//third_party/dawn`; `:1199-1202`: the Dawn sources of Graphite (`skia_graphite_dawn_sources`, `gn/graphite.gni:314`). **[R]**
- `skia/third_party/dawn/BUILD.gn`: `//third_party/dawn` is an action that runs `build_dawn.py`, which configures **Dawn's own CMake build** in `third_party/externals/dawn` for the targets `webgpu_headers_gen`, `dawn_proc`, `dawn_native` (`build_dawn.py:78`, `:88-120`) and links `libdawn_combined.a` (`BUILD.gn:155`). Its third-party locations are checked first and the script exits if one is missing: `abseil-cpp`, `egl-registry`, `glslang`, `jinja2`, `markupsafe`, `opengl-registry`, `spirv-headers`, `spirv-tools`, `vulkan-headers`, `vulkan-utility-libraries`, `webgpu-headers`, `swiftshader` (`cmake_utils.py:372-398`); `DAWN_EMDAWNWEBGPU_DIR` is explicitly "not synced by Skia" (`:411`). With `is_canvaskit` the include directories are emptied ("Emscripten includes its own WebGPU headers", `BUILD.gn:35-43`, `135-145`) but the action and the library stay. `cmake_utils.py:244` maps the OS `wasm` to a CMake system name `wasm`, and nothing else in the scripts treats it. **[R]**
- The `DEPS` of the fork lists none of those checkouts, not even `dawn` (`skia/DEPS`, 13 entries: brotli, d3d12allocator, expat, freetype, harfbuzz, icu, libjpeg-turbo, libpng, libwebp, vulkanmemoryallocator, spirv-cross, wuffs, zlib). **[R]**

Consequence: turning `skia_use_dawn` on, as the tree is, would try to build native Dawn for a system called `wasm` from a checkout that is not there. For the browser that is the wrong thing anyway: the WebGPU implementation is the browser's, and the module needs only `webgpu.h`, `webgpu_cpp.h` and the script glue, all of which the Emscripten port provides. The target `//third_party/dawn` has to be replaced, for this target, by a group that carries the port's compile flag and no library. That is a patch of one build file (or an overriding `gn` argument added to it). Tint is not needed in the module either: Graphite emits WGSL text through SkSL's code generator, and `//third_party/dawn:tint` is a dependency of the `skslc` tool only (`BUILD.gn:744-759`). **[R]**

So **the full Dawn checkout and its dependencies are not needed for the browser**. (For the record, if the native route were ever wanted: Dawn with the twelve dependencies above is, from general knowledge, well over 1 GB of checkouts; not measured. **[G]**)

### Skia's Dawn backend on Emscripten

`skia/src/gpu/graphite/dawn/` (29 files) and `skia/include/gpu/graphite/dawn/` (`DawnBackendContext.h`, `DawnGraphiteTypes.h`, ...). `__EMSCRIPTEN__` appears 78 times in the sources. Two kinds of branch are mixed under the one macro: **[R]**

1. **Platform**: things only native Dawn has. YCbCr descriptors (`DawnTextureInfo.cpp:28`, `DawnSampler.cpp:91`, `DawnCaps.cpp:632`, ...), `device.Tick()` (`DawnSharedContext.cpp:105`), `ProcessEvents` (`DawnBackendContext.h:52-56`), Dawn-only features (`DawnCaps.cpp:307-390`), asynchronous pipeline creation aborts on WebAssembly (`DawnGraphicsPipeline.cpp:705`, `819`). These are right for any Emscripten.
2. **Interface version**: the shape of Emscripten's *old* `webgpu.h`. `WGPUBufferMapAsyncStatus` and the old `MapAsync` signature (`DawnBuffer.cpp:18-89`, `244`), `wgpu::ShaderModuleWGSLDescriptor` where native has `wgpu::ShaderSourceWGSL` (`DawnSharedContext.cpp:23`, `DawnResourceProvider.cpp:36`, `DawnGraphiteUtils.cpp:502`), the deprecated `GetCompilationInfo(&Handler::Fn, ...)` (`DawnGraphiteUtils.cpp:472`), `wgpu::VertexStepMode::VertexBufferNotUsed` (`DawnGraphicsPipeline.cpp:627`, `653`), `wgpu::SupportedLimits` (`DawnCaps.cpp:281`), the old error scope callbacks (`DawnErrorChecker.cpp:44-133`), a work submission class "useful for wasm where wgpu::Future is not available yet" (`DawnQueueManager.cpp:18-98`, `127`), and version tests against Emscripten 3.1.48 and 3.1.51 (`DawnCaps.cpp:430`, `DawnCommandBuffer.cpp:27-44`, `DawnGraphiteUtils.cpp:427`).

The second kind does not compile against the pinned Emscripten: see section 3. The work is to move those dozen places (eight files) to the newer interface while keeping the first kind as it is. Whether the port's `webgpu_cpp.h` at its pinned version matches what the *native* branches of this Skia revision expect, name for name, is **[U]** until it is compiled; the two are developed together in Dawn, and Skia's fork does not record which Dawn revision it was tested with.

CanvasKit, which is where one would look for a working recipe, is no guide in this tree: `modules/canvaskit/compile.sh:49-53` and `:218-272` set `skia_use_dawn`, `skia_use_webgpu`, `skia_enable_graphite` and `skia_canvaskit_enable_webgpu`, and `modules/canvaskit/BUILD.gn:192-207` links with `-sUSE_WEBGPU=1`, `-sASYNCIFY` and the runtime methods `WebGPU,JsValStore`; but `canvaskit_bindings.cpp:106`, `368-414` still calls `GrDirectContext::MakeDawn` and `GrDawnTextureInfo` (Ganesh on Dawn), which no header of the tree declares any more. It cannot compile. `experimental/webgpu-bazel` is the same old interface. Skia's own Bazel module pins Emscripten 4.0.7 (`MODULE.bazel:6`), from before the port replaced the setting. **[R]**

### Asyncify or JSPI

Not required by Skia. `DawnBackendContext.h:24-49` describes it: with a tick function (which on the web has to yield to the event loop, hence Asyncify) the context can wait for the GPU; without one the context is "non-yielding", `Context::submit` with `SyncToCpu::kYes` is disallowed, and the client must see that GPU work is finished before it destroys the context (`hasUnfinishedGpuWork`). "Using a non-yielding Context makes it possible to build and run Graphite/Dawn on WebGPU without -s ASYNCIFY." `DawnCaps.cpp:438-446` then turns off CPU sync and asynchronous pipeline creation. **[R]**

Asyncify rewrites the whole module (the settings file calls its cost in size and speed "significant", `emscripten/src/settings.js:801-811`), and JSPI (`settings.js:920-927`; no longer experimental per `emscripten/ChangeLog.md:57`) depends on browser support **[G]** and on every export that can suspend being declared. The module is 11.7 MB with gzip today; neither is attractive. The plan below uses the non-yielding context and requests the adapter and device in script.

### Threads

Skia's generic WebAssembly configuration already compiles with `-pthread` (`skia/gn/skia/BUILD.gn:817-822`), which is why the published `libskia.a` could be kept for the threaded build and only the shim recompiled. A from-source build produces a `libskia.a` that is usable by both kinds of module in the same way **[R]** for the flags; that one archive really serves both is **[U]** (the build without threads links the published archive today, whose objects also carry the atomics feature).

## 3. Emscripten

- Pinned version: 6.0.10 (`scripts/browser/setup.sh:24`; `emscripten/emscripten-version.txt`). `docs/porting/browser-platform.md:50` gives the reason it cannot go lower: it is the floor of `wasm-bindgen` on this target. **[R]**
- `-sUSE_WEBGPU` was deprecated in 4.0.10 and **removed** in 4.0.18 "in favor of the external port Emdawnwebgpu" (`emscripten/ChangeLog.md:649-655`, `487-488`). Accordingly there is no `system/include/webgpu`, no `html5_webgpu.h`, no `libwebgpu.js` in the SDK (`emscripten/system/include/` has `GL`, `GLES*`, `EGL`, `webgl`, ...; `emscripten/src/lib/` has `libhtml5.js`, `libhtml5_webgl.js`). **[R]**
- The port: `emscripten/tools/ports/emdawnwebgpu.py`, a "remote port". It holds no sources: it names a package to download at first use, `https://github.com/google/dawn/releases/download/v20260423.175430/emdawnwebgpu_pkg-v20260423.175430.zip`, with its SHA-512 (lines 137-146). The package is not in the SDK's cache (`emscripten/cache/ports` does not exist). Usage: `--use-port=emdawnwebgpu` at compile and at link; the package "is text-only and does not contain any binaries"; "the sources include both C++ and JS code. While it is possible to precompile the C++ code to `.a`, the JS code cannot be precompiled and must be provided at the final link step"; closure is recommended for size (`--closure=1`). Parts of `webgpu.h` from `webgpu-headers` are called stable; "all of `webgpu_cpp.h`" is "NOT considered stable" (docstring, lines 9-130). The SDK tests it with and without `-sASYNCIFY` (`emscripten/test/test_other.py:12382-12386`). **[R]**
- **Workers and OffscreenCanvas.** The local SDK says nothing about WebGPU there. `OFFSCREENCANVAS_SUPPORT` and `OFFSCREEN_FRAMEBUFFER` (`settings.js:1795-1823`) are about WebGL contexts and Emscripten's own canvas proxying; FerroUI does not use them (it transfers the canvas itself: `webapp/modules/ferroui/rendering/webRenderTargetRegistry.ts`, `create` and `postCanvas`). How the port keeps its table of script objects with pthreads, and which helper imports a `GPUDevice` or `GPUTexture` that script created (the old bindings had `Module.preinitializedWebGPUDevice`, `emscripten_webgpu_get_device`, `emscripten_webgpu_import_texture`, `JsValStore`: `skia/modules/canvaskit/webgpu.js:7-20`, `74-91`) can only be read in the package: **[U]** locally. From general knowledge **[G]**: WebGPU is exposed in dedicated workers, `OffscreenCanvas.getContext("webgpu")` exists, WebGPU objects cannot be sent between threads, so the device, the canvas context and everything Skia makes from them must live on the one thread that renders; and the port exposes import functions on its `WebGPU` runtime object for objects created in script.

That last constraint fits the render worker as it is: the worker already owns the canvas and creates the WebGL context there, in the handler of `registerCanvas`.

## 4. FerroUI's side

### What exists

- `src/Skia/FerroUI.Skia/gpu/graphite/graphite_gr_context.rs` (236 lines): `GraphiteGrContext` over a `graphite::Context` and its recorder. Surfaces, flush (snap, insert, submit), upload of raster images with and without mipmaps, device-lost query. **Nothing in it is Metal-specific**; it implements the backend-neutral `ISkiaGrContext` (`gpu/i_skia_gr_context.rs`). It is compiled only for Apple (`gpu/mod.rs:9-10`: `#[cfg(target_vendor = "apple")] pub mod graphite;`). **[R]**
- `gpu/metal/skia_metal_gpu.rs` (267 lines) is the model for a new GPU. Metal-specific in it: `BackendContext::new(device, queue)` and `context_factory::make_metal` (lines 49-53), `backend_textures::make_metal` (178), the colour type `BGRA8888` (181), the auto-release pool (213, 265), and the Metal contracts of the port (`IMetalDevice`, `IMetalPlatformSurfaceRenderTarget`, ...). Everything else is the shape any Graphite GPU has: the shared context cell, `ISkiaGpu`, a render target that wraps the frame's texture as a surface, a session that flushes on dispose. **[R]**
- `gpu/open_gl/` is Ganesh over the OpenGL contracts, used by the browser through `rendering/browser_web_gl_render_target.rs` (`WebGlContext`, registered with Emscripten's `GL` object by `webGlRenderTarget.ts`).
- The build script mirrors the manifest's target table as `cfg(ferro_skia_ganesh_gl)` (`build.rs:20-25`); the manifest gives the browser `features = ["gl"]` (`Cargo.toml`, last lines). CI asserts that the target resolves to exactly `ganesh` and `gl` (`.github/workflows/ci.yml:89-94`), and the threads shim pins the key `ganesh-gl-jpegd-jpege-pdf` (`skia-threads-shim.sh`, `SKIA_FEATURES_KEY`). **[R]**

### What would be new in `FerroUI.Skia`

1. The `graphite` module compiled wherever Skia has Graphite (a `cfg` of the build script, like the Ganesh one), not only for Apple.
2. `gpu/dawn/` (name open): a `SkiaDawnGpu` after `SkiaMetalGpu`. A small contract for the platform side, in the role `IMetalDevice` has: the three handles (instance, device, queue), and a render target whose session hands out the current texture of the canvas and its size. The render session wraps that texture (`backend_textures::make_dawn`, `surfaces::wrap_backend_texture`) with the colour type of the configured canvas format, and flushes on dispose. No auto-release pool.
3. Readback. `GraphiteGrContext::snapshot_to_raster` (lines 172-187) uses the synchronous `read_pixels`, which a non-yielding context refuses. Its one caller outside tests is `surface_render_target.rs:175`. Either the browser's Dawn context answers `None` there (and whatever needs the snapshot takes another way), or an asynchronous readback is added whose result arrives on a later turn. What depends on that snapshot in the browser has to be audited first. **[U]**
4. `is_lost` already reads `Context::is_device_lost` (line 229-231). `SkiaMetalGpu::is_lost` returns `false` (line 93); the Dawn GPU must not.
5. `GraphiteGrContext::flush` calls `submit(None)`; with a non-yielding context somebody has to call `check_async_work_completion` regularly (once per frame is the natural place) so that finished work is released. Before the context is destroyed the work must be finished (`DawnBackendContext.h:43-45`): closing a view has to wait for a turn of the event loop, where today everything is released at once. **[R]** for the requirement; the design is open.

### What would be new in `FerroUI.Browser`

1. **Rendering mode.** `BrowserRenderingMode { Software2D = 1, WebGL1, WebGL2 }` (`browser_app_builder.rs:16-23`, mirrored in `webapp/.../renderingMode.ts`; a test asserts that `"WebGPU"` does not parse, line 201). A fourth value, `WebGPU = 4`, keeps upstream's numbers. Default order: unchanged at first (WebGL2, WebGL1, Software2D), WebGPU only when asked for (`?RenderingMode=WebGPU`), and put first only after it has been measured to be better.
2. **Script side**: a `WebGpuRenderTarget` beside `WebGlRenderTarget`. Unlike `getContext("webgl2")`, it cannot be constructed synchronously: `navigator.gpu.requestAdapter()` and `requestDevice()` return promises **[G]**. `WebRenderTargetRegistry.createRenderTarget` tries the modes in order in one synchronous pass today; with WebGPU in the list it becomes asynchronous, with fall-through to the next mode when there is no `navigator.gpu`, no adapter, or the device request fails. On the worker this costs nothing structurally: the canvas arrives by message and the worker reports the result with `OnRenderTargetRegistered(id, kind)` when it has one; the UI thread already treats a canvas as not ready until then (`browser_surface_shared.rs`, the target kind). **On the single-thread path** `create` returns the target at once (`webRenderTargetRegistry.ts`, `pthreadId === 0`); there the same report-later path has to be used, which is a real change of that path. The target then configures the canvas context (`context.configure({device, format, alphaMode: "premultiplied"})`) and imports the device into the port's table, in the role `GL.registerContext` has for WebGL.
3. **A third target kind**, `RENDER_TARGET_KIND_WEB_GPU = 3` (`interop/canvas_helper.rs:64-67`), through `browser_surface_shared.rs` (lines 279 and 366 treat the kind as one of two), `render_statistics.rs`, `render_worker.rs`, `render_target_browser_surface.rs`.
4. **Rust side**: `rendering/browser_web_gpu_render_target.rs` after `browser_web_gl_render_target.rs` (365 lines). No context to make current. Per frame: resize the canvas if the shared size changed (as `begin_draw` does, lines 170-175; a resized canvas needs no reconfiguration by the specification **[G]**), get the current texture from script, import it, hand it to the session, and release the handle when the session ends. The browser presents when the task returns; there is no swap call. One texture per frame per canvas, and Skia's surface for it must be released before the task ends, which the session's dispose already guarantees.
5. **Device loss.** `WebGlContext::is_lost` is a `TODO` that returns `false` (`browser_web_gl_render_target.rs:299-303`), so there is no model to copy. For WebGPU it cannot be skipped: `device.lost` resolves when the tab's GPU process restarts or the adapter goes away **[G]**. Minimum: the script side watches the promise and reports it; the GPU answers `is_lost`; the render target is recreated (new device, new Graphite context, all textures gone) or the view falls to the next mode.
6. **Readback for tests.** The browser tests that read pixels do it from the page (the capture scripts in `scripts/browser/`) rather than through Skia **[U]** (not audited here); a canvas configured for WebGPU can be drawn to a 2D canvas or captured by the test driver like any other **[G]**.

### Upstream

`/Users/wieslawsoltes/GitHub/Avalonia` at `16572aeff1` (2026-10-07): `src/Browser/Avalonia.Browser/BrowserAppBuilder.cs:13-30` has `Software2D`, `WebGL1`, `WebGL2` and the default order WebGL2, WebGL1, Software2D; the script enum is the same three; a search of `src/Browser` for `webgpu` and `wgpu` finds nothing. **[R]** A WebGPU mode is therefore an addition of this port: a row in `DEVIATIONS.md` (the mode, its number, the asynchronous creation of the target, the loss handling), and `browser-platform.md:102-106` and item 11 of its list (which say the browser needs Ganesh on GL and that Graphite must not leak above `FerroUI.Skia`) stay true and gain a second case.

## 5. Costs and risks

| Item | Evidence | Unknown |
|---|---|---|
| **Fork of the bindings** | `from_ptr` is `pub(crate)`; no feature, argument or shim for Dawn (section 1). Both crates patched in the workspace, and kept in step with each release taken later. | Whether the project would accept the change, and when. |
| **Patches on Skia** | A dozen interface-version branches in eight files; one build file (section 2). Carried as a patch set against the pinned tag, reapplied at each Skia bump. | Whether the result compiles against the port's pinned `webgpu_cpp.h` without further changes; whether a newer Skia has already moved to the port. |
| **From-source build of Skia** | Today nothing of Skia is compiled for this target; CI downloads a 15 MB archive (`browser-platform.md:50`). The threads shim compiles four files and runs `gn gen` only (`skia-threads-shim.sh`, header). A real build is `git-sync-deps` for the third-party checkouts, `gn gen`, `ninja` over about a thousand objects (the published `libskia.a` has 1020, `browser-render-worker.md:1091`) plus Graphite's, then the shim, then the binding generator with libclang. The crate writes into the cargo registry when it does this itself (`browser-render-worker.md:1120`). The earlier attempt failed at dependency sync (`browser-platform.md:35`); why is not recorded. | Build time on a CI runner (tens of minutes is the expectation **[G]**, not measured). The cause of the earlier failure. Whether one archive serves both kinds of module. |
| **How to ship the build** | The crate reads an archive from `SKIA_BINARIES_URL` (how the threads shim is delivered today, `build-browser.sh:226`). The same mechanism can deliver a self-built archive with its own key: built once per pin (crate version, Emscripten version, feature set, patch set) by a script like the shim's, cached in `.tools/` and in CI's cache, so that ordinary builds still compile nothing of Skia. | Whether the archive should be built in CI on demand or published once as a release asset of the repository. |
| **Module size** | The module is 11.7 MB with gzip for the catalog, either kind (`browser-render-worker.md:1467`); Skia with Ganesh and raster is 4.6 MB raw before framework code (`browser-platform.md:95`). | The size of Graphite with the Dawn backend and the port's glue, against Ganesh with the WebGL emulation. Not estimable from the sources. Graphite carries the SkSL-to-WGSL generator and its own pipeline code; Ganesh carries the GLSL generator and Emscripten's GL library in script. |
| **Ganesh must stay** | WebGL2 is the path every browser has, and the software mode needs only raster (`browser-platform.md:97`). No published binary combines Graphite and Ganesh (`browser-platform.md:295`), but a self-built one can (`skia_enable_ganesh` and `skia_enable_graphite` are independent arguments; `gn/skia.gni:193-206` only constrains Dawn and D3D). | Whether to ship one module with both engines (largest download for everyone, simplest fall-back: the next mode in the same module) or a third module chosen by the loader (as `combine-site.mjs` chooses between threads and no threads). With threads that makes four combinations; the site budget of Pages is checked per module (`browser-render-worker.md:1369`). A fall-back across modules after a failed device request means loading another module, which the loader does today only for a refused memory (`browser-render-worker.md:1303`). |
| **Browser coverage** | `browser-platform.md:97` (secondary sources, marked unverified there): Chrome and Edge on desktop, Safari 26, Firefox 141+ on Windows and 145+ on Apple Silicon; not Firefox on Linux or Android; Chrome on Linux limited. | Current state per browser and OS; WebGPU in workers and on OffscreenCanvas per browser; behaviour of each on the port's glue. Nothing local says. |
| **Fixed memory of the threaded build** | The threaded module has a memory that does not grow, 512 MB by default (`build-browser.sh:249-262`, `browser-render-worker.md`, "The decision on growth: fixed"). GPU textures and buffers of WebGPU live in the browser's GPU process, not in the module **[G]**; uploads are copied out of the module's memory. Graphite keeps CPU-side recordings and upload staging in the module. | Whether the peak moves. To be measured with `scripts/browser/catalog-memory.mjs`, as for Ganesh. |
| **Asynchronous start** | Section 4: the device comes from two promises. On the worker the existing report-later path absorbs it. | Time to first frame: adapter and device requests plus the first pipeline compilations (Graphite compiles pipelines on first use; asynchronous creation is off on WebAssembly, `DawnCaps.cpp:438-446`, so the first frames with new pipelines can stall). `scripts/browser/first-frame.mjs` and `frame-times.mjs` measure exactly this. |
| **Benefit** | None is established. Graphite on Metal is what the desktop runs, so one engine on both would narrow the difference in rendering between desktop and browser, and the `ganesh` module and the OpenGL contracts could in time serve fewer targets. | Whether frames are faster than Ganesh on WebGL2 for this framework's content. This is the question that decides whether the rest is worth doing, and stage 3 is the first point where it can be answered. |
| **Interface stability** | The port calls all of `webgpu_cpp.h` unstable (section 3), and Skia's Dawn backend uses the C++ header throughout. | Each bump of Emscripten (which pins the port's version) or of Skia can break the pair. The port can be pinned separately from Emscripten by passing a local port file (`--use-port=<path>`), which should be done. |

## 6. A staged plan

Each stage ends with a check; a "no" stops the work with what was learned written here.

### Stage 0: read what cannot be read locally (one small download)

Download the port's package (the zip named in `emscripten/tools/ports/emdawnwebgpu.py:140`, checked against its SHA-512) into `.tools/`, unpacked, nothing run. Read:

- `webgpu_cpp.h` against the twelve interface-version places of section 2: does the native branch of each compile against it by name?
- the script library: the table of objects with pthreads; the import functions for a device and a texture created in script; what it needs exported (`EXPORTED_RUNTIME_METHODS`); whether it assumes `Module.canvas` or the main thread anywhere.
- the port file's options and what it adds to compile and link.

Go/no-go: the Skia patch is a rename-level change and the port has an import path usable from a worker. If the header has moved away from what Skia m153's native branches use (missing types, changed callbacks), estimate the patch again before going on, or wait for a bindings release on a newer Skia.

Download: one zip, size not known locally (text only; expected to be small **[G]**).

### Stage 1: the smallest experiment, no Rust

A stand-alone C++ page, outside the workspace (in `.tools/` or the scratch area): Skia built from the tree with Graphite and the Dawn backend only, linked with a 60-line `main.cpp` that takes a device imported from script, makes a non-yielding Graphite context, wraps the canvas's current texture, clears it to a colour and submits. Run in the worker of a pthread build with a transferred canvas, as `render_worker_clear` does for WebGL, and on the main thread.

It needs:

- the Skia patch set (the build file of `third_party/dawn`; the interface-version branches);
- third-party checkouts through `tools/git-sync-deps` of the fork's `DEPS`: of the thirteen entries, the ones this configuration reaches (expat, freetype, libjpeg-turbo, libpng, zlib, wuffs at least; the script fetches all of them unless told otherwise). Size not measured; a few hundred MB is the expectation **[G]**;
- `gn` (Skia's `bin/fetch-gn`, from chrome-infra-packages, as the threads shim already does), `ninja` and `cmake` are installed (`/opt/homebrew/bin`), python3;
- `gn` arguments: those of `config.rs` and `emscripten.rs` for this target with `skia_enable_ganesh=false skia_use_gl=false skia_use_webgl=false skia_enable_graphite=true skia_use_dawn=true skia_enable_pdf=false`, and the port's flag in `extra_cflags`;
- link: `--use-port=<local port file>`, `-pthread`, the memory flags of the threaded build, no Asyncify.

It answers: does Skia's Dawn backend compile and link against the port (the main risk); does a frame appear from a worker on an OffscreenCanvas; how long the Skia build takes on this machine; how large `libskia.a` and a minimal module are (with `scripts/browser/wasm-size-report.py`); which browsers of the test set run it.

Go/no-go: a cleared canvas from the worker in Chrome, and the build time and size written down. No frame, or a patch that grows beyond the dozen places, stops here.

### Stage 2: the bindings

Fork `rust-skia` at the tag of 0.153.3: the `dawn` feature, the `gn` arguments, the shim functions, the `gpu::graphite::dawn` module, the asynchronous readback, and `backend_without_engine`. Build the archive for the key with a script after `skia-threads-shim.sh` (which becomes a from-source build for this key: sync, `gn gen`, `ninja`, shim with `-pthread`, binding generator), delivered through `SKIA_BINARIES_URL`. Port the stage 1 page to Rust as an example of `ferroui-browser` beside `render_worker_clear` (`render_worker_clear_webgpu`), using `skia-safe` only.

Decide here between one module with both engines and a separate module, from the sizes of stage 1 and of this build with Ganesh also enabled.

Go/no-go: the example renders from the worker through the Rust bindings; the archive builds reproducibly from the script; CI's feature check and the shim's pin are updated to the new key without changing the existing two modules.

### Stage 3: FerroUI's GPU and render target, behind the mode

`graphite` compiled for the target; `SkiaDawnGpu`; the script target, the kind, the mode `WebGPU = 4`, selectable only by request; fall-through to WebGL2 on any failure of creation. `themed_view` with `?RenderingMode=WebGPU`.

Go/no-go, measured against the same build on WebGL2 with the existing scripts: `frame-times.mjs` and `scroll-profile.mjs` (frame time), `first-frame.mjs` (start), `catalog-memory.mjs` (peak of the fixed memory), `module-sizes.mjs` (download), and the capture comparison for rendering differences. If frames are not better, or the start and size are clearly worse, the mode stays an opt-in experiment or is removed; this is the stage at which the benefit is known.

### Stage 4: completeness

Device loss and recreation; closing a view with unfinished GPU work; resize under load; readback (whatever stage 3's audit of `snapshot_to_raster` found); the single-thread path's asynchronous creation; the catalog's full tour in the browser tests for the new mode; Safari and Firefox.

Go/no-go: the catalog tests pass in the mode on the browsers of the test set, and a lost device recovers or falls back without a reload.

### Stage 5: default and CI

The archive in CI's cache or as a release asset; the Pages build; the loader's choice; the place of WebGPU in the default order; `DEVIATIONS.md`, `browser-platform.md`, `TRACKING.md`.

## 7. Unknowns, in the order they can stop the work

1. Whether Skia m153's Dawn backend compiles against the port's `webgpu_cpp.h` after the interface-version branches are moved, and how much else has drifted (stage 0 reads it, stage 1 proves it).
2. How the port behaves on a pthread with an OffscreenCanvas, and how a device and a texture made in script are imported (stage 0, stage 1).
3. Why the earlier from-source build failed at dependency sync, and how long a build takes (stage 1).
4. Size of Graphite with Dawn against Ganesh with WebGL, and of both together (stages 1 and 2).
5. Whether it is faster for this framework's content (stage 3).
6. What uses the synchronous snapshot in the browser, and what an asynchronous one costs above `FerroUI.Skia` (stage 3 audit).
7. Browser coverage today, in workers in particular (stage 1 onward, by running).
8. Whether later releases of the bindings add Dawn, which would remove the fork (not answerable from the local registry).

## Availability check (2026-10-09, from public sources)

Asked: is there a published Skia with Graphite and Dawn for WebAssembly, or another way than a build of our own?

- **The Rust bindings.** No published binary of `skia-bindings` has Graphite for any feature combination of the WebAssembly target, and `skia-safe` 0.153 wraps Graphite for Metal and Vulkan only. In the bindings' discussion of Graphite (January 2026) Dawn is named as a possible second step; no branch or pull request for it is linked. (<https://github.com/rust-skia/rust-skia/discussions/1243>, <https://crates.io/crates/skia-safe>)
- **CanvasKit with Graphite and Dawn exists as community builds**, not as an official one: `sankarru/skia-canvaskit-webgpu` builds `canvaskit.js` and `canvaskit.wasm` from Skia's main branch on GitHub Actions with a patch taken from the fork `open-pencil/skia` (BSD-3-Clause). A build takes one to three hours. What it publishes is the script API of CanvasKit: a finished module, not a static library that the Rust bindings could link. (<https://github.com/sankarru/skia-canvaskit-webgpu>, <https://github.com/open-pencil/skia>)
- **What the community builds change for us:** the patch set that makes Skia's Dawn backend compile for the browser exists and is maintained by others, so stage 1 of the plan above starts from their patches and their build recipe rather than from nothing. The fork of the bindings (the Dawn functions, the build arguments) and the from-source build remain.
- **Other ways to WebGPU that need no Skia build:** the Vello backend (critical path row 25, queued) renders through `wgpu`, which runs on WebGPU in a browser, and its `vello_hybrid` crate has a WebGL2 path; the project describes itself as alpha and the web as not a primary target. (<https://docs.rs/crate/vello/latest>)
- **Not found:** a WebGPU implementation other than Dawn that Skia's Graphite accepts (a `wgpu`-backed one was floated in the bindings' discussion, nothing exists).
