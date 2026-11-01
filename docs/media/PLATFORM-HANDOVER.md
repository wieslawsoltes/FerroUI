# Portable media: platform-agent handover

This handover is explicitly requested for agents working on the native Windows
and Linux machines. It records the current code, evidence and integration work.
The original implementation plan remains in chat. This is **not** a declaration
that the complete portable-media specification has been implemented.

## Starting revision and working rules

- Repository: FerroUI. Branch: `codex/portable-media-framework`.
- Coordinator worktree on the Mac:
  `/Users/danw/.codex/worktrees/portable-media-framework/FerroUI`.
- Fresh upstream starting revision: `8518d2cada13541ee6149c259f918cfe504ba6c4`.
- Foundation/Apple commit: `ce88d55f2bcc19ad28c0caa173a37d11a6a221bc`.
- iOS/Android native providers and GL jobs: `76b95ab51a37be126809bc15b5d43f5ee62aa5d9`.
- Browser/ownership commit: `b0692f13104ff55cdbddbe5d7e52955fee743f45`.
- Start from the branch tip containing this handover, rather than upstream main
  alone. These commits have not been pushed by this task. Transfer the branch or
  its Git bundle before starting work on another machine.
- Use a separate worktree and branch for each platform, such as
  `codex/portable-media-windows` and `codex/portable-media-linux`. Keep the other
  platform's files unchanged unless a shared-contract change is necessary;
  identify such changes clearly for integration.
- The specification is [PORTABLE-MEDIA-FRAMEWORK-SPECIFICATION.md](PORTABLE-MEDIA-FRAMEWORK-SPECIFICATION.md).
  [README.md](README.md) describes the implemented subset. Application migration
  and server/backend changes remain outside scope.
- Keep the `FerroUI.Media.*` family: Media is the shared base; Video, Camera and
  Qr are independent additions. Video must not depend on Camera or vice versa.
  Controls and native providers remain separate optional dependencies.
- Preserve author attribution. Set the committer **per commit** to
  `Wiesław Šoltés <wieslawsoltes@users.noreply.github.com>` and inspect the result.
  Do not modify global Git configuration. For a POSIX shell:

  ```sh
  GIT_COMMITTER_NAME='Wiesław Šoltés' \
  GIT_COMMITTER_EMAIL='wieslawsoltes@users.noreply.github.com' \
  git commit -m 'Describe the platform change'
  git log -1 --format='Author: %an <%ae>%nCommitter: %cn <%ce>'
  ```

  On PowerShell, set those two process environment variables around `git commit`
  and restore their previous values afterward. Do not substitute `git -c
  user.name=...`, which can also change the author.
- Reference techniques were audited at
  `3e9f2015702741c7efaca58fc78f140ab89edbcf`; see the Apple package notices.
  That revision may need a separate transfer if it is not in the receiving
  clone. Do not merge its entire implementation or inherit its test status.

Example transfer from a bundle into an existing upstream clone:

```sh
git bundle verify /path/to/portable-media-framework.bundle
git fetch /path/to/portable-media-framework.bundle \
  refs/heads/codex/portable-media-framework:refs/remotes/handoff/portable-media-framework
git worktree add -b codex/portable-media-windows ../FerroUI-media-windows \
  refs/remotes/handoff/portable-media-framework
```

Use the Linux branch/worktree name on that machine. The bundle's prerequisite is
the upstream starting revision above. There is no implementation-plan file to
copy or commit.

## Integration map

| Area | Current entry points |
| --- | --- |
| Immutable metadata, budgets, leases, actual completion, publication epochs and admission | `src/FerroUI.Base/rendering/composition/frames/` |
| Owning queued native handles | `src/FerroUI.Base/platform/leased_platform_handle.rs`; `composition_interop.rs` |
| Shared-frame surface update | `CompositionDrawingSurface::update_shared_frame_async`; `IPlatformRenderInterfaceImportedImage::acquire_shared_bitmap` |
| Per-draw lease retention | `src/Skia/FerroUI.Skia/leased_gpu_bitmap.rs` |
| Metal completion and direct import | `src/Skia/FerroUI.Skia/gpu/metal/`; `gpu/graphite/graphite_gr_context.rs` |
| Ganesh completion hook and GL borrowed texture import | `gpu/ganesh/ganesh_gr_context.rs`; `gpu/open_gl/gl_skia_external_objects_feature.rs`; `FerroUI.OpenGL/shared_frame_texture.rs` |
| Existing Vulkan importer requiring a new direct route | `src/Skia/FerroUI.Skia/gpu/vulkan/vulkan_skia_external_objects_feature.rs` |
| MediaPlayer/MediaSource/session and native provider traits | `src/FerroUI.Media.Video/player.rs` |
| Camera discovery/capture/session/photo and native provider traits | `src/FerroUI.Media.Camera/lib.rs` |
| Camera preview and weak/owned session attachment | `src/FerroUI.Media.Camera.Controls/` |
| Native EGL completion, retirement and render-thread frame jobs | `src/FerroUI.OpenGL/egl/egl_context.rs`; `src/FerroUI.Media.OpenGL/` |
| Android shared surface, Media3 video and CameraX capture | `src/FerroUI.Media.Android/`; `.Video.Android/`; `.Camera.Android/` |
| iOS leased IOSurface views on the active Metal device | `src/iOS/FerroUI.iOS/metal/external_frames.rs` |
| Shared presentation control and importer adapter | `src/FerroUI.Media.Controls/texture_presenter.rs` |
| Video presentation and accessible transport controls | `src/FerroUI.Media.Video.Controls/` |
| Scheduling, viewport/input mapping and planar projection contracts | `src/FerroUI.Media/renderer.rs` |
| Optional QR scheduling, results and borrowed luma | `src/FerroUI.Media.Qr/` |
| Audited platform examples | `src/FerroUI.Media.Apple`, `.Video.Apple`, `.Camera.Apple`, `.Browser`, `.Video.Browser`, `.Camera.Browser` |
| Independent applications | `samples/PortableMedia`; `samples/PortableMedia.Browser`; `samples/PortableMedia.Mobile` (including separate provider Gradle scripts) |
| Synthetic audiovisual fixture | `tests/media/fixtures/avc-aac.mp4`; generator `tests/media/make-fixture.swift` |

Paths abbreviated with `gpu/` in this table are under `src/Skia/FerroUI.Skia`.
The existing composition system remains the renderer integration boundary.

### Ownership and synchronization requirements

1. Reuse a `FrameBudget` across source/size generations so old allocations and
   retiring GPU reads remain charged. Defaults are three allocations, 128 MiB,
   8192 maximum dimension and six GPU read reservations. They are configurable
   safety limits, not accepted performance thresholds.
2. Reserve capacity before creating new framework GPU storage. A retained native
   decoder buffer is an immutable owner, not permission to reuse its pixels.
   Count unique storage honestly and document opaque driver/decoder overhead.
3. Producer readiness, render-job completion and actual GPU completion differ.
   Retain inputs before submitting reads, and outputs before submitting writes.
   Signal completion only from the actual consuming queue/fence. Cancellation
   cannot release submitted storage. Missing completion quarantines bounded
   resources rather than inventing success.
4. `FrameRetirement` can retain accounting through owning-thread destruction.
   `LeasedPlatformHandle` retains source ownership across queued native imports;
   it does not replace consumer GPU-completion leases.
5. Add a lease-backed `acquire_shared_bitmap` implementation. Its default fails;
   passing through a legacy snapshot importer does not implement direct sampling.
   Use the current renderer's device identity and generation, rejecting stale or
   incompatible frames even when the physical adapter identity is unchanged.
6. Framework GPU preview must not map/read back/repack/stage/re-upload pixels.
   A required GPU operation needs a reason and counters. Known software graphics
   is rejected by GPU-only. GPU-preferred may use a separately declared bounded
   CPU fallback; that never closes a missing primary GPU gate.
7. Drop/coalesce preview work on saturation without blocking the UI. Retire
   queued output on seek/stop. Source replacement must reject late publication.
   Do not release caller-owned sessions when detaching their views.

`TextureRendererHost` already retains shared input frames before calling the
client renderer and awaits actual completion before publication. Client-owned Metal and GLES plane examples now consume those same source
leases. Native smoke probes verify transparent projection coverage and teardown;
clients still own scenes, rendering libraries and hit testing.

### Provider contracts

Video implements `MediaPlayerProvider::open` and `NativeMediaSession`: actual
state, play/pause/stop, native achieved seek, audio controls and shared frames.
Core code handles preparation-time intent, superseded commands, source epochs,
looping and caller ownership. `has_pending_frame_work` keeps paused/ended
presentation polling while an import/fence or a bounded native retry slot is
pending. Do not advance source sequence admission on budget saturation. Desktop
streams retain at most one additional retry sample, replacing it with newer
native output. Its default native `stop` pauses; override it when
pending producer output needs retirement. The player separately requests zero
for seekable media. Report real seek ranges, not merely a nonempty range list.

Camera implements `CameraProvider` and `NativeCameraSession`. Discovery must not
acquire hardware or prompt. `start` is explicit and receives the budget to use.
Canceling a permission future must also stop a stream that arrives later. Report
negotiated dimensions and fractional frame rates, actual consent/disconnection,
and unavailable controls honestly. Return independently owned JPEG output and
native-still versus preview-snapshot provenance. Stop clears pending preview
output, while already submitted GPU leases retire through completion.

## Windows agent

Provider source is now present in `FerroUI.Media.Windows`,
`FerroUI.Media.Video.Windows` and `FerroUI.Media.Camera.Windows`. The
`PortableMedia` desktop harness selects these providers on Windows. Native
C++ compilation and all native verification are deferred to this host; a Rust
FFI check on `x86_64-pc-windows-gnu` passed with native compilation explicitly
skipped. A full cross-build of the sample could not find the Windows C compiler.
This is not evidence that its native SDK bridge compiles or runs.

Read [NATIVE-DESKTOP-PROFILES.md](NATIVE-DESKTOP-PROFILES.md) before building.
The bridge selects the active ANGLE D3D11 device, probes compatible video and
threading interfaces, configures an MF DXGI device manager and advanced RGB
processing, and rejects system-memory video samples. It retains MF samples,
texture slices and readiness queries through native alias retirement. WASAPI
provides native audio output/clock. The UI backend first attempts a BGRA/video
capable D3D11 device, preserving its ordinary graphics fallback when unavailable.

Native EGL/Ganesh consumer completion and the lease-backed GL importer now
exist. WGL, legacy snapshot imports and Vulkan do not gain direct media sampling
from this change. The selected native output profile is SDR RGB32 with validated
color, aperture, size and adapter identity. Missing native tags are rejected;
verify that the requested MF processor actually negotiates those tags and
compatible shader-readable DXGI storage on each selected codec/driver.

Start validation with an independent D3D11 producer and deterministic pixels,
then native playback and camera. Cover compatible device management, WARP/software
rejection, incompatible resources, actual delayed completion, two consumers,
resize/replacement/device loss, native audio, overlapping seeks, permissions,
hotplug and owned JPEG capture. Record the required Windows packaging and native
keyboard/UI Automation evidence separately from implementation.

## Linux agent

Provider source is now present in `FerroUI.Media.Linux`,
`FerroUI.Media.Video.Linux` and `FerroUI.Media.Camera.Linux`. The desktop harness
explicitly selects EGL/GLES on X11. Rust FFI/provider checks passed on the
`x86_64-unknown-linux-gnu` target with native compilation explicitly skipped.
The GStreamer/EGL C bridge and portal/PipeWire C bridge have not been compiled,
linked or run on Linux. Fix native integration failures on this host before
claiming a functional provider.

The selected profile uses system GStreamer 1.24+, exact DMA_DRM format/modifier
caps from the active EGL display, retained samples and bounded native slots.
Compatible XRGB/XBGR frames create direct EGL aliases. NV12 uses the required
counted external-texture GPU conversion; it currently needs GLES3 and known
BT.709 tags. Unknown/implicit layouts, unsupported color, crop, interlacing and
pixel-aspect conversions fail admission. No CPU preview fallback is declared.
GLX and Vulkan direct sampling remain implementation work for the broader
primary-renderer gates; do not report a successful EGL profile as those routes.

Portal access is in the Camera crate, so Video has no portal/PipeWire development
dependency. Discovery is metadata-only, consent is explicit, canceled requests
close their portal handles, and only authorized nodes are enumerated. Enumeration
and capture each consume a fresh portal remote connection. Direct V4L2 is an
explicit configured-device profile and never a denial fallback. Camera stills
are independently owned JPEG preview snapshots with counted Photo transfers;
modifier-backed snapshots outside the linear profile fail honestly.

Validate independent DMA-BUF production first, then negotiated playback/capture,
missing plugins, incompatible modifiers/formats/devices, software adapters,
denial/hotplug, actual synchronization, bounded old-generation resources and
independently decoded photos. Record the session/compositor, renderer, driver,
GStreamer/plugin versions, portal backend and device profile with the evidence.

## Evidence already obtained

These are tests of this implementation, not inherited media-branch results.
They are functional evidence, not a frozen performance acceptance profile.

| Check | Result and limit |
| --- | --- |
| Windows/Linux provider adapters | Rust cross-checks passed with the native bridge deliberately skipped; C/C++ compilation, linking, codec/driver behavior and native samples remain unverified |
| Android container geometry | A pre-fix reflected-track negative control exposed Media3's missing reflection. Bounded inspection of the existing encoded stream now preserves the full orthogonal transform and integral clean aperture. The 16 KiB emulator passed independent projected colours, natural size, seek/Stop and source/renderer retirement for the reflected/cropped fixture; zero preview CPU transfers. Host JVM contracts separately cover fragmented/ranged reads, extended boxes, malformed metadata, cancellation and output-crop mapping. |
| Apple playback geometry | macOS and iOS simulator runs decoded rotated/mirrored and off-centre clean-aperture fixtures. The native crop `(10,6,144,80)`, natural size, cardinal rotation/reflection and independent projected quadrant colour probes passed, with zero preview CPU transfers and final source/renderer retirement. Two Rust geometry contracts separately exercise affine point mapping and rejection. |
| Paused producer saturation | Android 16 KiB and iOS 27.0 simulator runs retained three real source leases across a paused seek, observed pending work after 1.5 seconds, then published the sought frame after release. Stop/frame-zero and zero retained source frames passed; preview CPU counters stayed zero. A UI contract test separately checks polling beyond the settling interval and idle shutdown. |
| Integration sample restart | Camera restart preserves the requested 3D state across temporary source absence. Android 16 KiB run observed replacement capture generation 3 feeding a new renderer generation, continuing QR/JPEG, then Stop with renderer frames zero and no camera-service clients. Strict transformed-video smokes passed again on Android and iOS. |
| Renderer scheduling | Four mailbox notification/lifetime tests and two UI scheduling tests cover worker dispatch, disposal, static idle, ancestor visibility and continuous work. macOS/iOS native playback/Metal projection and Android 16 KiB playback/GLES projection and teardown passed again. Android camera + QR + JPEG also continued through rotation and released hardware/output after Stop. |
| Native alias contracts | Three GL producer/retirement unit tests passed, including retention through consumer completion, queued native cleanup and lost-context quarantine; this does not prove a native DMA-BUF/DXGI import |
| Android renderer accessibility | Updated APK's native automation tree reports RendererHost focusable; camera stop again released hardware and renderer storage. Full keyboard/screen-reader acceptance remains open. |
| Browser regression after renderer changes | Actual Chrome suite passed shared WebGL, native video, synthetic capture/JPEG, canceled acquisition, software rejection, autoplay/audio and CORS cases again |
| Targeted Rust contracts | 62 tests across frame/admission/retirement, queued-handle ownership, existing surface jobs, player ordering/lifetime, camera contracts, QR scheduling/adapter, geometry, renderer host and native fixtures; runs are recorded separately |
| Independent Metal producer | Real GPU rendering and completion delay, direct sampling, opaque/premultiplied/Rec.709 pixels, stale-generation rejection, native texture ownership and counted diagnostic reads |
| Native macOS playback | Synthetic AVC/AAC resource → shared Metal frame → pause → achieved seek → resume → Stop/frame zero → clear; zero preview CPU transfers and zero retained frames after cleanup |
| Native camera discovery | Zero devices on this Mac; permission stayed NotDetermined; no acquisition |
| Native camera teardown fixture | Foundation-only fake session/delegate lifecycle; no camera hardware |
| Browser sample | Real Chrome/ANGLE Metal/Skia WebGL2; independent quadrants in two controls, sibling ordering, native video pixels, nonblocking fences, delayed completion notification and retirement after detach |
| Browser media/capture | File resources, native seek/overlapping seek/Stop frame zero, autoplay denial and gesture recovery, diagnostic decoded AAC audio samples, CORS rejection, discovery without acquisition, denial, synthetic camera and independently decoded JPEG, late permission cancellation |
| Browser negative controls | CPU-backed preview upload rejected/counted before frame allocation; known SwiftShader adapter rejected; normal production preview counters remain zero |
| Browser tooling | TypeScript/lint/bundle passed; 60 build-tooling tests passed, including the repaired macOS symlink entry-point check |
| iOS native providers | Device-target compilation at deployment target 14.0; independent mobile app built, signed, installed and run on iOS 27.0 simulator. Shared Metal playback → pause → seek 750 ms → resume → Stop/frame zero → detach passed with zero preview CPU transfers and zero retained frames. Simulator camera discovery returned no devices. Physical camera/audio and minimum-OS runtime gates remain open. |
| Android foundation | Native EGL render-thread jobs, nonblocking GL fences and deferred native surface retirement. On an API 35 ARM64 emulator with a verified 16,384-byte kernel page size, one independent texture fed two views and retired; zero preview CPU transfers and zero GPU copies. |
| Android camera | Separate CameraX 1.5.2 provider and camera preview controls. Emulator permission denial/grant, live GPU preview, native JPEG independently decoded at 1856×1392, and Stop returning camera-service active clients to an empty list. The request-owned/background-copy version also produced a decoded native JPEG and rejected capture completion after Stop. Host JVM contracts check independent photo bytes and late cancellation. |
| Renderer host | Normal composition control plus client-owned Metal/GLES adapters and projected-plane examples. macOS and iOS 27.0 simulator runs checked transparent coverage/alpha, playback/seek/Stop and renderer-frame retirement. Android GLES ran the corresponding video/plane and counted diagnostic pixel probe on the 16 KiB emulator. Pure Rust/JVM contracts cover input replacement, detach/reattach, queued-job rollback, unknown completion, crop/orientation/mirror and planar QR projection. |
| Native QR | macOS Vision decoded two synthetic symbols from retained CoreVideo input; iOS 27.0 simulator did so using the revision-1 CPU analysis profile. Android bundled ML Kit 17.3.0 decoded the same two symbols and checked geometry/raw bytes with network services disabled on the 16 KiB emulator. CameraX live analysis ran at about 10 Hz, dropped excess frames and allowed a separately decoded native JPEG; Stop returned active camera clients to an empty list. Native SDK internal copies remain opaque. |
| Android playback | Separate Media3 1.8.0 provider with native decoder/audio timing, a bounded presentation-time table, coalesced commands and a dedicated player looper. API 35/16 KiB run passed independent GPU sharing, playback, pause, achieved seek, resume, Stop/frame zero, frame retirement and native surface/player cleanup. Earlier failing runs remain separate evidence; fixes address surface-clock mapping, async command settling and obsolete queued acquisition. |

The development machine is an Apple M4 Mac, macOS 27.0.1, with the iOS SDK
27.1 and the pinned browser Rust 1.99.0/Emscripten 6.0.10 tools. Current Xcode's
libc++ headers warn for the older iOS deployment target; `cargo check` does not
establish minimum-OS linking/runtime support. Preserve the framework's iOS 14
floor and verify packaging with an appropriate toolchain rather than raising
the floor to silence that warning.

Local raw logs are under
`/Volumes/SSD/build-cache/ferroui-portable-media-evidence/`; they are not repository
files or automatically available on another machine. Important names include
`browser-integration.log`, `browser-webapp.log`, `browser-tooling-tests.log`,
`queued-handle-contract.log`, `browser-foundation-contracts.log`,
`browser-video-contracts.log`, `browser-surface-regression.log`,
`native-metal-generation-regression.log`, `native-browser-regression-smoke.log`,
`apple-ios14-check.log`, `apple-native-qr-tests.log`, `ios-native-qr-smoke.log`,
`ios14-native-qr-check.log`, `android16k-offline-qr-smoke.log`,
`android16k-live-qr-camera-counters.log`, `native-qr-core-contracts.log`,
`macos-renderer-smoke.log`, `ios-renderer-smoke.log`,
`android16k-renderer-smoke.log`, `renderer-source-contracts.log`,
`renderer-native-job-contracts.log`, `renderer-geometry-contracts.log`,
`android16k-camera-qr-renderer.log`, `android-unknown-color-negative.log` and
`android-frame-orientation-contracts.log`, `native-alias-contracts.log`,
`windows-rust-bindings-check.log`, `linux-rust-bindings-check.log`,
`windows-sample-bindings-check.log`, `desktop-gl-scene-bindings-check.log`,
`desktop-provider-routing-smoke.log`, `ios-final-renderer-smoke.log`,
`frame-notification-contracts.log`, `renderer-notification-ui-tests.log`,
`renderer-notification-macos-smoke.log`, `renderer-notification-ios-smoke.log`,
`renderer-notification-android-smoke.log`, `renderer-notification-camera-qr.log`
and `renderer-notification-browser-regression.log`, `pending-frame-ui-tests.log`,
`pending-frame-contracts.log`, `pending-frame-ios-saturation.log` and
`pending-frame-android-saturation.log`, `pending-frame-macos-smoke.log`,
`pending-frame-browser-regression.log`, `pending-frame-windows-bindings.log` and
`pending-frame-linux-bindings.log`, `apple-track-crop-pixels-macos.log`,
`apple-track-crop-pixels-ios.log`, `apple-geometry-affine-contracts.log` and
`video-geometry-android-negative.log`, `android-mp4-geometry-pixels.log`,
`android-mp4-geometry-contracts.log`, `android-output-crop-contracts.log`,
`android-mp4-saturation-regression.log`, `android-mp4-camera-qr-regression.log`,
`mobile-navigation-android-camera.log`, `mobile-navigation-android-video.log` and
`mobile-navigation-ios-video.log`. Re-run applicable gates on the receiving host.

Useful commands, from the repository root:

```sh
cargo test -p ferroui-base --lib rendering::composition::frames
cargo test -p ferroui-base --lib shared_frame_handle_tests
cargo test -p ferroui-base --lib composition_drawing_surface_tests
cargo test -p ferroui-media -p ferroui-media-video -p ferroui-media-camera --lib
cargo test -p ferroui-media-controls -p ferroui-media-video-controls -p ferroui-media-qr --lib
cargo test -p ferroui-skia --lib --features native-frame-tests native_shared_metal_frame
cargo test -p ferroui-media-camera-apple --lib --features native-lifetime-tests
cargo run -p portable-media -- tests/media/fixtures/avc-aac.mp4 --smoke --resource
scripts/build-browser.sh portable-media-browser --debug
FERROUI_BROWSER_ANGLE=metal node scripts/browser/tests/portable_media.test.mjs
```

The native Metal/camera/application and browser commands require their corresponding host/tools.
Use an appropriate hardware ANGLE backend on Windows/Linux; do not turn the
primary GPU test into a SwiftShader pass. The browser suite tests software
rejection independently. Activate the pinned browser tools first. On the Mac,
single-job builds with incremental compilation and debug information disabled
were used to limit disk/memory use; the target cache is private to this worktree.

## Work still open beyond these handoffs

- Windows and Linux provider source is implemented for the explicitly described
  profiles, with native C/C++ compilation, packaging and runtime verification
  deferred. Broader primary-renderer routes remain open.
  Android now has native playback/CameraX/external GLES integration and a
  separate active-media/camera run on a 16 KiB kernel.
- Apple Vision and bundled Android ML Kit QR engines are implemented. Physical
  camera geometry/encoding and the wider native QR fixture matrix still need
  verification. The optional Rust adapter does not expose complete payload bytes,
  and its internal input-copy behavior still needs profiling. Do not infer zero
  copying from a borrowed input interface or zero framework transfer counters.
- `FerroUI.Media.Camera.Controls` now provides CameraPreview with explicit
  start, owned-session teardown and weak caller-session attachment. Complete
  markup/source APIs and automatic device-loss recovery remain unfinished.
  RendererHost, native Metal/GLES adapters and client-owned video/camera plane
  examples now exist; wider input/accessibility and physical geometry gates
  remain open.
- Wider Apple geometry/color profiles, physical capture/denial/hotplug, minimum
  OS linking/runtime and the broader browser/device matrix need
  separate evidence. Browser native frame output currently requires a page-owned
  single-threaded WebGL2 context, frame callbacks and matching decoded/upload
  dimensions; it rejects CPU-backed uploads. Worker media bridges are deferred.
- All per-profile packaging, native keyboard/accessibility and frozen hardware/
  workload/performance acceptance gates remain open where not explicitly tested.

Return implementation commits, exact build/run commands, observed native
capabilities, counters and unresolved failures. Keep implementation status,
simulated tests, hardware evidence and performance acceptance distinct.
