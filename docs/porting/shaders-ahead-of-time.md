# Shaders compiled ahead of time on every platform (#786)

Issue #786 asked whether the shaders of the GPU backends can be compiled when an application is
built and shipped with it, so that a first start compiles nothing. The Metal part is done and
merged: on macOS a Graphite context compiles the pipelines of a frame side by side on a thread
pool (section 9.81 of `desktop-performance.md`), makes the pipelines of a shipped record of keys
when the platform starts (section 9.83), and loads the Metal libraries of a shipped table where
it would compile them from source (section 9.85; `gpu-rendering.md`, "Pipelines made ahead of
their first draw"). iOS carries the same libraries, compiled for iOS (section 3). This document
is the investigation of the rest: what each backend and platform would need, what the Skia
(Graphite and Ganesh) and the Vello side offer, and what it would save, and the plan for the
Ganesh platforms (section 7). Nothing in it is a measurement unless it names where the number
was measured.

## 1. What a first start pays, and where

A GPU pipeline is made in two steps: the *front end* turns the text of a shader into the
intermediate form of the platform, and the *back end* turns that into code for the GPU and a
pipeline state. Which of the two a build step can take over depends on whether the platform has a
portable intermediate form.

| Step | Metal | Vulkan | Direct3D (ANGLE on Windows) | OpenGL, OpenGL ES, WebGL |
| --- | --- | --- | --- | --- |
| Front end | MSL text to AIR (`newLibraryWithSource:`) | SPIR-V is the input: none at run time if SPIR-V is shipped | HLSL to DXBC/DXIL (`D3DCompile`, inside ANGLE) | GLSL text to the driver's form, inside the driver |
| Back end | AIR to GPU code (the pipeline state) | SPIR-V to GPU code (`vkCreate*Pipelines`) | DXBC to GPU code, inside the driver | inside the driver (`glLinkProgram`) |
| Portable form that can be shipped | Metal libraries (`.metallib`), by GPU family and system for binary archives | SPIR-V; a `VkPipelineCache` blob is of one driver | DXBC (ANGLE has no entry point for it) | none: a program binary is of one driver and one GPU |
| Kept by the system between runs | yes, by the system's shader cache | by the driver, where it keeps one (Mesa and the vendors' drivers do by default) | by the driver for the D3D part; ANGLE's own program cache only when the application gives it a blob cache | by the driver where it keeps one; on Android the system gives each application a blob cache |

The measured share on macOS (section 9.85, the threads of the pipeline pool on a first start):
313 of 424 samples of making a pipeline were `newLibraryWithSource:`, the front end, and the rest
the pipeline state.

## 2. The backends and what they offer

### Skia, Graphite (macOS, iOS; the WebGPU browser prototype)

- **Keys.** Graphite reports every pipeline it makes with a key that can be stored
  (`ContextOptions::fPipelineCallback`) and makes a pipeline from such a key
  (`PrecompileContext::precompile`). The record of ordinary drawing is 37 pipelines in 2.3 kB
  (#786, second pass). Two kinds stay out of a record in the pinned Skia: pipelines that sample a
  texture (the key of the coordinate-normalising snippet does not read back) and Skia's own
  runtime effects (refused until a context has drawn with them). The paint-options form of the
  precompile API (`Precompile(context, PaintOptions, DrawTypeFlags, RenderPassProperties)`) builds
  those keys as a draw does, but needs Skia's headers, which only the patched binding has.
- **Shader text.** Graphite writes one vertex and one fragment text per pipeline at run time and
  hands it to the device in one place (`MtlCompileShaderLibrary`). The table of Metal libraries
  answers below that, on the device (`gpu/metal/shader_libraries.m`), so it also serves the
  texture pipelines and the runtime effects that a record cannot hold.
- **Persistent storage.** `PersistentPipelineStorage` is implemented for Vulkan only; nothing in
  the pinned Skia uses Metal's binary archives.

### Skia, Ganesh (Windows on ANGLE, Linux on OpenGL or Vulkan, Android, the browser on WebGL 2,
and Apple targets built with `ferro_skia_apple_ganesh`)

- **Persistent cache.** `GrContextOptions::fPersistentCache` is the one hook: Ganesh asks it for
  a program by a key before it compiles and gives it what it compiled. With
  `fShaderCacheStrategy = kSkSL` the cache holds Skia's own shading language, which is device
  independent, and `GrDirectContext::precompileShader(key, data)` compiles such an entry ahead of
  its first draw (the workflow Skia documents in `GrDirectContext.h`). With `kBackendBinary` it
  holds the program binary of the driver (`glGetProgramBinary`), which is of one driver; on
  Vulkan the entry is the `VkPipelineCache` data.
- **What the binding offers.** skia-safe 0.153 has `ContextOptions::shader_cache_strategy` and
  `DirectContext::precompile_shader`, and keeps the cache field private
  (`context_options.rs`: "TODO: PersistentCache"). `ContextOptions` has the layout of
  `GrContextOptions` (it is transmuted), so a small C++ file of this crate could set the field, as
  `gpu/graphite/pipeline_executor.cpp` sets up Graphite's executor. The cache itself is a class
  with virtual members whose arguments are `SkData` and `SkString`, which a file without Skia's
  headers would have to declare with their layout (the data pointer and the size of `SkData` are
  read by inline members). The executor needed only a factory and a destructor; this needs the
  layout of two classes of Skia, which the published binaries do not guarantee. On the targets
  other than macOS the binding takes Skia's published binaries and has no headers.
- **Today** the project sets no persistent cache on any Ganesh context, so Skia keeps nothing
  between runs; what is kept is whatever the driver or the system keeps by itself.

### Vello (every platform, through wgpu)

- **A closed set.** vello 0.11 compiles all its compute pipelines when its renderer is made
  (`Renderer::new`), for the antialiasing modes asked (`RendererOptions::antialiasing_support`;
  `vello_gpu_scene_sink.rs` asks for one mode per renderer). The WGSL of the set is in the crate;
  naga translates it to MSL, SPIR-V, HLSL or GLSL when the pipeline is made, and the driver
  compiles that.
- **Threads.** `RendererOptions::num_init_threads` compiles the set on several threads; vello's
  default is one thread on macOS and a heuristic elsewhere. `vello_gpu_scene_sink.rs` keeps the
  default.
- **Pipeline cache.** `RendererOptions::pipeline_cache` takes a `wgpu::PipelineCache`. In wgpu 30
  the feature `PIPELINE_CACHE` is of Vulkan only; its data is of one driver, so it speeds the
  starts after the first, not the first.
- **The passes of FerroUI** (`gpu/effect_passes.rs`, the lighting, the layers with an effect, the
  shader effects of #455) are render pipelines made on the first frame that draws them, once per
  device; their compile time has not been measured.
- **Metal under wgpu.** wgpu makes its Metal libraries with `newLibraryWithSource:options:error:`
  of the device. The front of `shader_libraries.m` is put on the class of the device Skia draws
  with, so it is in a process that links `ferroui-skia`; a Vello program does not link it. The
  same table would serve Vello if the front moved to a crate both link and a table were recorded
  from Vello's texts (naga's output is fixed for one version of wgpu and one kind of device).

### ProGPU (#369)

A retained engine on WebGPU (wgpu-native on the desktop, the browser's WebGPU in the browser)
with a fixed set of WGSL shaders, like Vello's: what applies to Vello applies to it, through its
own engine repository.

## 3. By platform: what is done, what it would need, what it would save

| Platform | Backend | Done | What it would need | What it would save |
| --- | --- | --- | --- | --- |
| macOS | Graphite on Metal | the thread pool (9.81), the record of keys (9.83), the table of libraries (9.85) | the pipeline states of the texture pipelines and the runtime effects ahead of their first draw: the paint-options precompile, in the patched binding | the back end of those pipelines (a quarter of a pipeline); measured first frame of a first start, with the table: 1.018 s against 1.246 s without (section 9.85) |
| iOS | Graphite on Metal (Ganesh on Metal with `ferro_skia_apple_ganesh`) | the thread pool and the front, as on macOS; the crate carries the table of ordinary drawing compiled for iOS devices (`ordinary-ios-apple.fmz`) and for the simulator (`ordinary-ios-simulator.fmz`), made of the texts recorded on the Mac (section 5) | a record of keys made on iOS (whether Skia refuses a key of another device or fails on it is not known, so the macOS record is not used there); the libraries of an application's own screens, recorded on iOS and registered (`register_metal_libraries`) | the front end of a first start after an install, as on macOS; not measured on a device |
| Windows | Ganesh on OpenGL ES (ANGLE) | nothing kept by Skia | a persistent cache on disk with the `kSkSL` strategy, and `precompileShader` of a shipped set at start; ANGLE's own program cache on disk needs a blob cache given through `EGL_ANDROID_blob_cache`, which no platform code of this repository gives | every start after the first: the compile of SkSL to GLSL to HLSL and the D3D compile. A first start only from a shipped SkSL set, and it still pays the front and back end, off the first frame |
| Linux | Ganesh on OpenGL, or Vulkan with the feature | nothing kept by Skia; the drivers keep their own disk caches | the same persistent cache; on Vulkan its entries are `VkPipelineCache` data, of one driver | Skia's part of the starts after the first (the drivers already keep theirs); a first start only from a shipped SkSL set |
| Android | Ganesh on OpenGL ES | nothing kept by Skia; the system gives each application a blob cache of program binaries | the same persistent cache, mainly for the SkSL set compiled ahead | Skia's generation and the work before the driver; the driver's part is cached by the system after the first start |
| Browser | Ganesh on WebGL 2 (Graphite on WebGPU as a prototype) | nothing: the browser compiles, and keeps compiled programs in its own cache | nothing can be shipped (WebGL takes source only); a shipped SkSL set compiled ahead at start through `precompileShader`, with `KHR_parallel_shader_compile` where the browser has it | the stall of the first draws, moved before the first frame; not the work |
| Every platform | Vello | the whole set is made with the renderer | a table of libraries for Metal as above; a `wgpu::PipelineCache` on Vulkan; more initialisation threads on macOS (to be measured, vello's default is one) | not measured: the set is small and is made once, before the first frame |

## 4. The one mechanism, by layer

The requirement of #786 is one way to record, to ship and to compile ahead on every platform,
with the best form of the platform underneath. From the table above:

1. **Record.** A run of the application writes what it compiled: Graphite keys
   (`FERROUI_PIPELINE_RECORD`), Metal texts (`FERROUI_METAL_LIBRARY_RECORD`), and on Ganesh the
   SkSL entries a persistent cache is given (to be added with the cache).
2. **Ship.** A file of the crate or of the application, registered before the first window, as
   `register_pipeline_record` and `register_metal_libraries` are; an asset where the program
   should not grow.
3. **Compile ahead.** When the platform starts, on threads, before the first frame: Graphite's
   precompile on its pool (done), `precompileShader` on Ganesh (to be added), and on Metal the
   front end answered from the table (done).
4. **Keep between runs.** The persistent cache of Ganesh on disk (to be added) serves every start
   after the first, which no platform but Apple's and Android's keeps for Skia today.

## 5. What is within reach from the code, and what is not

- **iOS: the tables of the crate.** Everything the macOS route uses is compiled for iOS. The
  build script of the crate takes the table of the kind of device of the target when its file
  is there (`tables_of_the_target`: `gpu/metal/libraries/ordinary-<platform>.fmz` and
  `gpu/graphite/records/ordinary-<platform>.fpr` for `macos-apple`, `ios-apple` and
  `ios-simulator`), and `scripts/perf/metal-libraries.py make <platform> <texts>` makes one in
  one step. The iOS libraries are compiled from the texts recorded on the Mac: Skia writes a
  text from the capabilities of the device (`MtlCaps`; nothing of the text generation depends on
  the system it is built for), the GPUs of the Apple family from Apple7 on (A14, M1) have the
  same shader capabilities on both systems (framebuffer fetch, the number of varyings, clamp to
  border), and the device of the simulator is the GPU of the Mac. The simulator loads only
  libraries made for it (`-mios-simulator-version-min`; a library made with the minimum flag of
  the devices is refused there, as one of iOS is on macOS), hence the two tables. A text of
  another device (an older iPhone) is compiled as before. A record of keys for iOS is a file away
  once one is made on iOS and taken whole there.
- **Ganesh: a persistent cache, not without Skia's headers.** It needs a C++ subclass of
  `GrContextOptions::PersistentCache` handed to the options of every place that makes a Ganesh context
  (`gpu/open_gl/gl_skia_gpu.rs`, which the browser's WebGL context goes through too,
  `gpu/vulkan/vulkan_skia_gpu.rs`, and `gpu/metal/skia_metal_gpu.rs` for Ganesh on Metal). Declared by hand, as the executor is, it would depend on the
  layout of `SkData` and `SkString` in binaries this project does not build; that is not a draft
  to put behind a feature without a run on each platform. The sound route is a hunk of the
  patched binding (`external/rust-skia-bindings`) that binds `PersistentCache` with Skia's
  headers, which reaches the platforms only when the binding builds Skia from source for them
  (it does so for macOS today) or a patched binding is published. Section 7 is the plan.
- **Vello: options of its own.** `num_init_threads` and `pipeline_cache` are fields of
  `RendererOptions` and need no patch; whether more threads help on macOS is a measurement
  (vello's own default chose one thread there).

So macOS and iOS have the route; the Ganesh platforms need the binding first (section 7).

## 6. To measure, by platform

- **iOS** (a device, not the simulator): the first frame of a first start after an install, with
  the table of the crate and with `FERROUI_METAL_LIBRARIES=off`. In the simulator, whether the
  texts a run records (`FERROUI_METAL_LIBRARY_RECORD`) are in the table
  (`metal-libraries.py check`), and that every library of it loads
  (`the_device_loads_every_library_of_the_table_of_ordinary_drawing`).
- **Windows, Linux, Android** (the VMs and the emulator, `platform-benchmark-gates`): the first
  frame of a first and of a second start, before a persistent cache exists, to know what the
  driver already keeps; then with the cache on disk.
- **Browser** (Chrome): the time of the first draws of the catalog with and without
  `KHR_parallel_shader_compile` and with a shipped SkSL set compiled ahead.
- **Vello** (macOS): `Renderer::new` with one and with several initialisation threads on a cold
  Metal cache.

## 7. The plan for the Ganesh platforms

Windows (ANGLE), Linux (OpenGL, Vulkan), Android and the browser (WebGL 2) draw with Ganesh.
Ganesh has one place for all of it: `GrContextOptions::fPersistentCache`, which it asks for a
program by a key before it compiles one and gives what it compiled, on OpenGL, Vulkan and Metal
(`GrGLProgramBuilder`, `GrVkPipelineStateBuilder`, `GrMtlPipelineStateBuilder`), and
`GrDirectContext::precompileShader(key, data)`, which compiles an entry of such a cache ahead of
its first draw (OpenGL and Metal). The plan, in the order of the work:

1. **The cache, bound by the binding.** `GrContextOptions::PersistentCache` is a class with
   virtual members (a destructor, `load(const SkData&)` returning `sk_sp<SkData>`, two `store`
   overloads), so a subclass has to be compiled with the headers of the Skia it is linked with:
   the order of the virtual table, the destructor's slots and how an `sk_sp` is returned are
   those of that build. A hunk of the patched binding (`external/rust-skia-bindings`, next to the
   Graphite Metal fix) adds a glue class to its sources (`src/ganesh.cpp`) that forwards `load`
   and `store` to two functions and a context pointer of Rust, and a setter of the private field
   `persistent_cache` of `skia_safe::gpu::ContextOptions` (which has the layout of
   `GrContextOptions`). Declaring the class by hand in this crate, as
   `gpu/graphite/pipeline_executor.cpp` declares the executor, is not the plan: the executor
   needed a factory and a destructor; this needs the layout of a class whose virtual members
   take Skia's objects, on compilers that differ in how they return a class by value.
2. **Getting the glue to the platforms.** The glue of the binding is compiled into the archives
   it downloads for these targets (`skia-bindings` among the libraries of `binaries_config.rs`),
   and the binding builds from source only for macOS today. Either its build script also builds
   the targets of the Ganesh platforms from source (a Skia build per target on CI, as for macOS),
   or it compiles the one glue file of the hunk against the headers of the pinned revision
   (`m153-0.101.2`, fetched without building Skia) beside the downloaded archive. The second is
   cheaper and must use the defines of the downloaded build (`SK_TRIVIAL_ABI` and the GPU
   backends); a test that stores and loads one entry through a real context on each platform
   proves the layout.
3. **The cache in Rust** (`ferroui-skia`, a module of `gpu/ganesh`): a map from key to data in
   memory, filled when the first context is made from the entries the application ships and from
   the file the previous run wrote, and given what Ganesh stores. The file is written off the
   render thread after the first frames and when the process ends, in the cache folder of the
   platform (`%LOCALAPPDATA%` on Windows, `$XDG_CACHE_HOME` on Linux, the application's cache
   folder on Android; the browser keeps it in memory only, its own program cache is the
   browser's). A file starts with the revision of Skia and the vendor, renderer and version
   strings of the driver, and a file of another driver is dropped. The format follows the record
   of keys: a magic, then the length and the bytes of each key and of each entry.
4. **The strategy.** A context has one: `kBackendBinary` (the driver's program binary, of one
   driver and one GPU) for the cache on disk, which serves every start after the first, where
   the driver has program binaries (`glProgramBinary`; ANGLE has them over Direct3D); `kSkSL`
   (Skia's own shading language, device independent) for a recording run that makes the set an
   application ships. Vulkan stores its `VkPipelineCache` data and SPIR-V through the same
   cache.
5. **A first start.** The shipped SkSL set (`FERROUI_SKSL_RECORD=<file>` writes one, a script
   lists and merges them, an application registers one, as it registers a record of Graphite)
   is compiled with `precompileShader` when the context is made, before the first frame. A
   context of OpenGL is of one thread, so this moves the compile before the first frame rather
   than off the thread; the browser compiles in parallel where it has
   `KHR_parallel_shader_compile`. Vulkan has no `precompileShader`: its first start gains
   nothing from a shipped set.
6. **Where it is set.** Every place that makes a Ganesh context: `gpu/open_gl/gl_skia_gpu.rs`
   (Windows, Linux, Android and the browser go through it), `gpu/vulkan/vulkan_skia_gpu.rs`, and
   `gpu/metal/skia_metal_gpu.rs` in the configuration `ferro_skia_apple_ganesh` (where the table
   of libraries already answers the front end). One switch for all of them, off for a measure
   (`FERROUI_SHADER_CACHE=off`), as `FERROUI_METAL_LIBRARIES=off` is for the table.
7. **Measured before it is kept**, on the gates of each platform (the VMs, the emulator,
   Chrome): the first frame of a first and of a second start without the cache, to know what the
   driver already keeps by itself (Mesa and the vendors' drivers keep a disk cache, Android gives
   each application a blob cache); then the second start with the cache on disk, and the first
   start with a shipped set. A platform where the cache saves nothing over the driver's own is
   left out with its numbers.
