# Linux native verification — 2026-11-01

Starting revision: `6aac17e56e81e3ecd2b14113fc02ea0979be0f59` from
`origin/media/portable-media-framework`. Work is isolated on
`media/portable-media-linux`. Native compilation is enabled throughout; the
binding-check-only switch is unset.

## Environment

Ubuntu 26.04.1 under WSL2, kernel `6.18.33.2-microsoft-standard-WSL2`, Rust/Cargo
1.93.1, GStreamer 1.28.2, GLib/GIO 2.88.0, PipeWire 1.6.2 and Mesa 26.0.8.
The host GPU is an NVIDIA GTX 1660 Ti. Default EGL selects llvmpipe; explicitly
setting `MESA_LOADER_DRIVER_OVERRIDE=d3d12 GALLIUM_DRIVER=d3d12` selects
`D3D12 (NVIDIA GeForce GTX 1660 Ti)` with GLES 3.1.

The hardware display does **not** advertise `EGL_EXT_image_dma_buf_import` or
its modifier extension. `/dev/dxg` is present, but there is no `/dev/dri` or
camera device. The installed GStreamer VA plugin exposes zero decoder features.
Hardware GL rendering alone therefore does not establish this DMA-BUF media
profile. A Linux host with a compatible DRM/EGL display and decoder/camera output
is still required for the positive GPU gates.

## Fixes and checks

- Correct the GStreamer header to `gst/video/video-info-dma.h`.
- Emit system link dependencies after the static native bridges. The original
  ordering failed the native test executable under the linker's `--as-needed`.
  The media and portal bridges use the corrected ordering.
- Validate a linear plane's full row extent against both its FD allocation and
  visible GstMemory range. A pre-fix negative control accepted an eight-byte FD
  for a 64-byte plane. The new native test checks FD retention/release, four
  invalid extents/strides, and system-memory rejection. It uses memfd descriptors
  for metadata tests; it does not pretend those are genuine DMA-BUF GPU imports.

The full `portable-media` sample and both C bridges compile and link natively.
The native descriptor test passes. The shared media, camera, GL ownership and
video suites pass **40 tests** (5 + 7 + 3 + 25).

The actual hardware-display sample rejects playback with
`No explicitly negotiated native DMA-BUF formats` and exits 1. The llvmpipe
negative control rejects playback with
`Software/unknown GL has no admitted GPU media route` and exits 1. Portal camera
discovery returns zero devices, unchanged NotDetermined permission and
`acquired=false`, exiting 0. No preview CPU fallback was added.

## Reproduction

Install system development/runtime dependencies: GStreamer core/base development
packages and codec/plugins (including libav and PipeWire), GLib/GIO Unix, PipeWire,
EGL/GLES, fontconfig/freetype, X11 development libraries, Vulkan headers, a C/C++
compiler, libclang, CMake and Ninja. No codec is silently downloaded by the app.

From the worktree root in Ubuntu:

```sh
export CARGO_TARGET_DIR=~/build-cache/ferroui-portable-media
export CARGO_BUILD_JOBS=4 CARGO_PROFILE_DEV_DEBUG=0
cargo build --locked -p portable-media
cargo test --locked -p ferroui-media-linux --features controls,native-frame-tests --lib -- --nocapture
cargo test --locked -p ferroui-media -p ferroui-media-video -p ferroui-media-camera -p ferroui-media-opengl --features ferroui-media-opengl/controls --lib
export MESA_LOADER_DRIVER_OVERRIDE=d3d12 GALLIUM_DRIVER=d3d12
eglinfo
cargo run --locked -p portable-media -- tests/media/fixtures/avc-aac.mp4 --resource --smoke
cargo run --locked -p portable-media -- --camera-discovery
LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe cargo run --locked -p portable-media -- tests/media/fixtures/avc-aac.mp4 --resource --smoke
```

The two playback commands are expected rejection tests on this WSL display.
When launching from PowerShell use `wsl -d Ubuntu --exec bash -lc '…'`, so the
requested Bash owns expansion and exit-status reporting.

Raw logs are outside Git, beside this worktree in
`<checkout>\`: `linux-build-final.log`,
`linux-native-frames-final.log`, `linux-media-contracts.log`,
`linux-playback-d3d12.log`, `linux-software-rejection.log`,
`linux-camera-discovery.log` and `linux-environment.log`.

## Remaining acceptance

Real DMA-BUF pixels/modifiers, implicit fences, NV12 conversion, actual consuming
GPU completion, two consumers, device loss, portal consent/cancellation/hotplug,
direct V4L2, independently decoded camera JPEGs, native audio, accessibility,
clean-machine packaging and frozen performance workloads remain unverified.
WSL rejection evidence closes none of those positive GPU or camera gates.

The synthetic-camera follow-up on Windows also identified that frame polling
could wait on a pending import before observing a native pipeline failure. Linux
now checks native session state first too, including during budget saturation.
Its native build and descriptor tests are rerun for this change; positive Linux
GPU/camera behaviour still needs a DMA-BUF-capable host. The Windows in-process
fixture does not enable Linux camera emulation or change this WSL capability.

The subsequent Windows camera-recovery/presentation work also reruns the native
WSL sample build successfully (`linux-camera-recovery-regression.log`). Its
Windows-only fixture/control diagnostics add no Linux preview fallback and
close none of the positive DMA-BUF gates above.

The subsequent Windows soak exposed a shared compositor ownership leak:
dropping a render-bound interop handle on the UI thread retained the old backend
and device. `LockBound` now queues destruction for its owning render thread and
wakes that thread. The WSL render-thread suite passes all 15 tests, including
UI/third-thread release and stopped-loop confinement; the native sample rebuild
also passes (`linux-owner-drop-tests.log`, `linux-geometry-final-build.log`).
This shared lifecycle fix adds no DMA-BUF capability or CPU preview fallback.

The Windows verification runner subsequently caught an asynchronous EOF restart
race in the shared `MediaPlayerElement`. Its polling timer now gives an old
Ended snapshot the same bounded command-settling allowance as Paused, while
settled EOF still stops polling. All nine video-control contracts pass natively
under WSL, including a deterministic delayed-state regression
(`linux-ended-control-tests.log`). The Windows verification runner can also run
this suite and rebuild the native Linux sample with `-IncludeWsl`; these are
control/build results and do not establish DMA-BUF presentation.

The subsequent native-memory work changes shared EGL cleanup: a context whose
initial make-current fails is destroyed immediately, a failed switch restores
the prior context, and disposing a current context explicitly unbinds it so
EGL cannot defer releasing its native resources. The WSL mock EGL suite and
five factory/display-ownership contracts pass. An opt-in test also creates a
real EGL/GLES display/context under WSL, makes it current, disposes it and
observes `EGL_NO_CONTEXT` (`linux-native-egl-disposal.log`). It passes while
Mesa reports that the X server has no DRI3 device. The final renderer string
is `llvmpipe (LLVM 21.1.8, 256 bits)` (`linux-native-egl-disposal-final.log`).
The final shared EGL suite passes 27 tests, with this opt-in test ignored in
that ordinary invocation and separately run successfully. This verifies native EGL
lifetime behavior, not accelerated rendering or DMA-BUF media import.

```sh
cargo test --locked -p ferroui-opengl --lib \
  native_current_context_is_unbound_on_disposal -- --ignored --nocapture
```

The Windows-side ANGLE device cache does not change Linux device selection.
The remaining positive Linux media/camera gates still require another host or
a WSL graphics stack with the required DMA-BUF extensions.

The 2026-11-02 integration of GitHub main `df8b92d94` adopts upstream
`Subscription` guards in Linux native polling and the shared media controls.
Pending native futures keep their wakeup timer alive and drop it on cancellation;
the existing context-retirement and EGL cleanup paths are preserved. WSL passes
the sample type-check, nine context-manager tests, 15 render-thread tests, five
renderer-scheduling tests and the native descriptor test. The real EGL-disposal
test also passes with llvmpipe. Logs use the `media-main-merge-` prefix beside
the worktree. These integration results add no DMA-BUF or physical-camera gate.
The shared EGL fence poll now uses `GL_SYNC_FLUSH_COMMANDS_BIT` with a zero
timeout after Windows exposed a stalled idle import. WSL reruns all 28 EGL
tests including native disposal successfully (`media-main-merge-linux-egl-flush.log`).
