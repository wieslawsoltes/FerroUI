# Linux native verification — 2026-11-01

Starting revision: `6aac17e56e81e3ecd2b14113fc02ea0979be0f59` from
`origin/codex/portable-media-framework`. Work is isolated on
`codex/portable-media-linux`. Native compilation is enabled throughout; the
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
export CARGO_TARGET_DIR=/home/danw/build-cache/ferroui-portable-media
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
`C:\Users\Dan\.codex\worktrees\portable-media-linux\`: `linux-build-final.log`,
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
