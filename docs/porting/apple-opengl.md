# OpenGL on the Apple platforms

The rendering mode OpenGL of the macOS backend (CGL, `src/Avalonia.Native/AvaloniaNativeGlPlatformGraphics.cs` upstream) and of the iOS backend (EAGL, `src/iOS/Avalonia.iOS/Eagl/` upstream), and the build of the Skia backend that draws with them.

Marks, as in the other platform documents: **[V]** verified from sources, **[M]** measured here (a build, a test or a run on the development Mac: Apple silicon, macOS 26, Xcode 26.4.1), **[S]** shown by a run in the iOS simulator, **[R]** recalled and not verified.

## 1. What upstream does

| Platform | Rendering modes, default order | OpenGL graphics | Surface |
|---|---|---|---|
| macOS (`AvaloniaNativePlatformOptions.RenderingMode`) | Metal, OpenGL, Software; a mode that fails to initialize is passed over, `Software` ends the list | `AvaloniaNativeGlPlatformGraphics`: one CGL context made to read the version (OpenGL 3.2 core profile asked for by the native side), then the shared context every top level renders with (`UsesSharedContext`) | `GlPlatformSurface`, the first surface of a top level (`[Gl, Metal, framebuffer]`); the native side renders a frame into an IOSurface the view shows |
| iOS (`iOSPlatformOptions.RenderingMode`) | Metal, OpenGL; an empty list, or a list none of whose modes applies, throws | `EaglPlatformGraphics`: one EAGL context (OpenGL ES 3.0) shared by the whole application; `CreateContext` is not supported | `EaglLayerSurface` over the `CAEAGLLayer` of the view, whose layer class follows the graphics |

**[V]** (the upstream files at the tracked commit; the native files `cgl.mm` and `rendertarget.mm`, which the port already had: `Avalonia.Native_native_sources.md`).

## 2. Skia: no published binary has Ganesh on OpenGL next to Graphite

The Skia backend of the port draws on Apple systems with **Graphite on Metal**, the one GPU back end of the default build (`DEVIATIONS.md`, Skia backend). Upstream's SkiaSharp draws with Ganesh on Metal and Ganesh on OpenGL and has both in one library. The binaries of `skia-bindings` 0.153.3 that the release publishes for Apple targets, asked with a one-byte range request for each key **[M]** (2026-10-15):

| Features (key) | `aarch64-apple-darwin` | `x86_64-apple-darwin` | `aarch64-apple-ios` | `aarch64-apple-ios-sim` |
|---|---|---|---|---|
| `graphite-jpegd-jpege-metal-pdf` (the default build) | yes | yes | yes | yes |
| `ganesh-gl-jpegd-jpege-metal-pdf` | yes | yes | yes | yes |
| `ganesh-jpegd-jpege-metal-pdf` | yes | yes | yes | yes |
| `ganesh-gl-jpegd-jpege-pdf` | yes | yes | yes | yes |
| `jpegd-jpege-pdf` (raster) | yes | yes | yes | yes |
| `ganesh-gl-graphite-jpegd-jpege-metal-pdf` | no | no | no | no |
| `ganesh-graphite-jpegd-jpege-metal-pdf` | no | no | no | no |
| `ganesh-gl-graphite-jpegd-jpege-pdf` | no | no | no | no |

So Ganesh on OpenGL and Graphite cannot be in one build without compiling Skia from source, which the project does not do (`vulkan.md`, section 1: hours and gigabytes). Cargo features are additive and cannot take `graphite` away, so the choice is a configuration of the compiler:

- **Default build**: Graphite on Metal, as before. The rendering mode OpenGL still creates its platform graphics (the platform does not know the render backend), and the Skia backend refuses the OpenGL context at the first frame with the message of upstream's backend for a context it does not know ("Graphics context of type is not supported"). The default order (Metal first) never gets there on a system with Metal.
- **`RUSTFLAGS="--cfg ferro_skia_apple_ganesh"`**: the manifest of `ferroui-skia` has two target tables for Apple targets, `cfg(all(target_vendor = "apple", not(ferro_skia_apple_ganesh)))` with `graphite` and `metal`, and `cfg(all(target_vendor = "apple", ferro_skia_apple_ganesh))` with `gl` and `metal` (which gives `ganesh` too). Cargo evaluates the configuration of a target table with the flags of the compiler, so the flag picks the table and the binary `ganesh-gl-jpegd-jpege-metal-pdf` is downloaded **[M]** (`cargo tree -e features` shows `ganesh`, `gl`, `metal`, `jpeg`, `pdf` with the flag and `graphite`, `metal`, `jpeg`, `pdf` without it; the build printed no source build). The build script turns the configuration into `ferro_skia_ganesh_gl` for Apple targets, which compiles the OpenGL GPU of the backend (`gpu/open_gl`, the same files as on Windows, Linux, Android and the browser), and the Metal GPU (`gpu/metal`) is compiled on Ganesh: `SkiaMetalGpu` with a direct context of Metal (`direct_contexts::make_metal`), the drawable's texture as a Ganesh render target with the origin of the session, the stencil option and the limit of the resource cache as upstream's `SkiaMetalGpu.cs`, and the imported images of the external objects feature wrapped as Ganesh render targets with their origin, as upstream's `SkiaMetalExternalObjectsFeature.cs`. That build is upstream's own arrangement (Ganesh for both), so the rendering mode is a run-time choice between Metal and OpenGL, as upstream has it.

The flag applies to every crate of the build, so a change of it rebuilds everything; a build directory per configuration avoids that (`CARGO_TARGET_DIR`). The graphite tests of the Metal GPU (`gpu/metal/tests.rs`) are compiled in the default build only.

## 3. macOS: the CGL graphics (`ferro_native_gl_platform_graphics.rs`)

Upstream file `AvaloniaNativeGlPlatformGraphics.cs`, eight classes, ported in its order:

| Upstream | Port | Notes |
|---|---|---|
| `AvaloniaNativeGlPlatformGraphics` | `FerroNativeGlPlatformGraphics` | `new` creates a context, reads `GL_MAJOR_VERSION`, `GL_MINOR_VERSION` and `GL_CONTEXT_PROFILE_MASK` with it current, makes the entry points (`dlsym` of the native display), then the display object and the shared context. A failure is an `Err` (the exception upstream's platform catches). `uses_shared_context` is true; `create_context` makes a context that shares nothing, as upstream. `Send + Sync` by an argument written next to the `unsafe impl`, the one of the Metal graphics (`render-thread.md`, R5.5). |
| `GlDisplay` | `GlDisplay` | The native display, the entry points, samples and stencil bits of the first context, the factory and the memory management helper (asked for once, as for Metal). `create_shared_context` returns the error of the native call. |
| `GlContext` | `GlContext` | The native context in a cell that `dispose` empties (lost). `make_current` gives the object of the native side that unlocks the context (`CGLLockContext` in `MakeCurrent`) and restores the previous one when it is released. `is_shared_with` is upstream's comparison by identity. The context announces itself as `IGlContext`, its handle wrapping feature and its external objects feature. |
| `GlPlatformSurfaceRenderTarget`, `GlPlatformSurfaceRenderingSession`, `GlPlatformSurface` | the same | The surface holds the native top level in `SurfaceTopLevel` (the UI thread only) and is not ready off the UI thread, as the Metal surface (`DEVIATIONS.md`, Native backend). The session reports `is_y_flipped` true. |
| `GlExternalObjectsFeature` | the same | Imports an IOSurface as a rectangle texture (`CGLTexImageIOSurface2D` through the native context) and a Metal shared event as a timeline semaphore (`ImportMTLSharedEvent` of the factory). The feature holds its context weakly. |
| `MtlEventSemaphore`, `ImportedTexture` | the same | `ImportedTexture::dispose` deletes the texture when the context can be made current and leaves it otherwise, as upstream's empty catch. |

Wiring, as upstream: `FerroNativePlatform` tries `ObtainGlDisplay` and the graphics for `FerroNativeRenderingMode::OpenGl` and goes on with the next mode on a failure; the top level's surfaces are `[gl, metal, framebuffer]`. The default order of the options is upstream's (Metal, OpenGL, Software) and is unchanged.

**The factory in the display object.** Upstream keeps the factory for the import of shared events, which a frame of either thread may ask for. The port keeps it too: `ImportMTLSharedEvent` has no state, the reference count of the factory is atomic (the patch of `comimpl.h`, R5.4), and the reference is never the last one, because the platform binds the factory in the service locator of the UI thread for the life of the process. So the destructor of the native factory, which must run on the UI thread, never runs from the graphics.

## 4. Verification on macOS

- `cargo test -p ferroui-native` in both configurations: six tests of the CGL graphics over the real native display, the OpenGL of the system, a real IOSurface and a real shared event of the Metal device (the version is 3.2 core or later, one shared context, the sharing of contexts and of contexts made from a shared one, a lost context, the names and refusals of the external objects feature, an IOSurface imported as a rectangle texture, a shared event signalled and waited for as a timeline semaphore). See section 6 for the counts.
- `examples/platform_window.rs -- --opengl` in the configuration `ferro_skia_apple_ganesh`: the platform chooses the CGL graphics (OpenGL 4.1 core on this machine), a frame of the window's OpenGL render target cleared to (51, 102, 153) with OpenGL and read back before it is presented gives exactly that colour (1280 by 800 pixels at scaling 2, y flipped), the window is painted by Skia through Ganesh on the CGL context, and with `FERROUI_SMOKE_CAPTURE` the scene drawn into an offscreen layer of the render backend (a GPU surface of the same context) is written as PNG **[M]**.
- The ControlCatalog with `FERROUI_SMOKE_RENDERING=opengl` (macOS options of the desktop entry point, OpenGL alone; the run prints which graphics the platform applied) in the configuration `ferro_skia_apple_ganesh`: a screenshot run of the catalog (section 6) **[M]**.

## 5. iOS: the EAGL graphics

See `ios-platform.md`, section 7, for the files `eagl/eagl_display.rs`, `eagl/eagl_layer_surface.rs` and `eagl/layer_fbo.rs`, the layer class of the view and the threads. The same configuration of the Skia backend (`ferro_skia_apple_ganesh`) gives Ganesh on OpenGL ES there; the binary for both iOS targets is in the table of section 2.

## 6. Runs

All on 2026-10-15, on the development Mac, branch `apple-completion` **[M]**. Pictures and logs under `<workspace>/ferroui-screenshots/macos-gl/`; each is a frame the application draws, never a capture of the screen.

| Run | Build | Result |
|---|---|---|
| `cargo test -p ferroui-native` | default and `ferro_skia_apple_ganesh` | 125 passed in each, the six tests of the CGL graphics among them |
| `cargo test -p ferroui-skia` | `ferro_skia_apple_ganesh` (with `RUSTDOCFLAGS` for the doctests) | 467 passed, 9 ignored; the Graphite tests of the Metal GPU are not compiled in this build |
| `examples/platform_window.rs --opengl`, `FERROUI_SMOKE_EXIT_MS=2500`, `FERROUI_SMOKE_CAPTURE` | `ferro_skia_apple_ganesh` | Exit 0. OpenGL 4.1 core; the probe frame (1280 by 800 pixels at scaling 2, y flipped) read back (51, 102, 153, 255), the colour it was cleared to; a paint by Skia through Ganesh on CGL; `platform_window-opengl.png` is the scene drawn into an offscreen layer of the backend (a GPU surface of the CGL context) |
| The same, before the patch `0002` of the native library | `ferro_skia_apple_ganesh` | The run passed and the process crashed at its exit, three of three, in `PlatformRenderTimer::OnTick` (a display link tick on a freed timer); `--metal` in the default build exited cleanly, three of three. With the patch: exit 0, three of three |
| ControlCatalog, `FERROUI_SMOKE_RENDERING=opengl`, `FERROUI_SMOKE_SCREENSHOTS` | `ferro_skia_apple_ganesh` | Exit 0; "Platform graphics: OpenGL (CGL)"; Home, Buttons, TextBlock, ListBox, Image, Calendar and Slider shown and written (`catalog-opengl/`, the frames the framework draws of the window) |
| ControlCatalog, `FERROUI_SMOKE_RENDERING=metal` | `ferro_skia_apple_ganesh` | Exit 0; "Platform graphics: Metal" (Metal on Ganesh); the same seven pages (`catalog-metal/`), the same pictures as with OpenGL |
| iOS simulator (iPhone 17 Pro), smoke mode, `FERROUI_IOS_RENDERING_MODE=OpenGl`, run by the orchestrator on `69462a42` | `ferro_skia_apple_ganesh` | **[S]** Passed, no check failed: the OpenGL ES graphics applied, 8 frames presented on the EAGL layer, the frame read back (1206x2622) with exactly the colours drawn; the Metal mode of the same build (Metal on Ganesh) passed too; the catalog's host shown in the OpenGL ES mode, four pages (pictures under `<workspace>/ferroui-screenshots/ios-gl/`); `ios-platform.md`, section 12, run 10 |
| `cargo check` and `cargo test -p ferroui-ios`; the example and the catalog host for `aarch64-apple-ios-sim` | both | Built for both iOS targets in both configurations; 104 host tests and the ten checks of the dispatcher pass |

Not verified: the OpenGL mode of macOS driven by hand (live resize, several windows sharing the context, popups), the external objects feature of the CGL context with a producer of another API (the tests import a real IOSurface and a real shared event, without a picture drawn into either), a device build of iOS (the runs are in the simulator).

