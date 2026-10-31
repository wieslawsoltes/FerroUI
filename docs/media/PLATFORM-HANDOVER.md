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
client renderer and awaits actual completion before publication. A native 3D
client example still needs implementation; the contract's tests are not that
example or evidence of an implemented scene engine.

### Provider contracts

Video implements `MediaPlayerProvider::open` and `NativeMediaSession`: actual
state, play/pause/stop, native achieved seek, audio controls and shared frames.
Core code handles preparation-time intent, superseded commands, source epochs,
looping and caller ownership. Its default native `stop` pauses; override it when
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

Introduce the corresponding `FerroUI.Media.Video.Windows` and
`FerroUI.Media.Camera.Windows` packages; add a shared `FerroUI.Media.Windows`
helper only where ownership/import code is reusable. Reuse the common controls.
Do not add a CaptureElement compatibility control or port the full WinRT model.

The selected backend is Media Foundation with compatible D3D11 device management,
retained native output surfaces and native audio/timing. Establish how the active
renderer exposes/shares its D3D11 device, including adapter LUID and generation,
before integrating decode. A new private decode device with silent copies is
not the preferred route. Media-engine frame-server transfers must not become the
default primary path merely because they are easier to capture.

The common Ganesh hook currently has a working completion provider only for the
browser profile. Native contexts still opt out. Add native completion on the
actual consuming context/queue and direct native-image acquisition; do not infer
native Windows support from browser GL passing. Check default ANGLE/D3D paths
and explicitly identify any narrower renderer profile. Necessary format/color
conversion must stay on the GPU and retain its input storage until completion.

Start validation with an independent D3D11 producer and deterministic pixels,
then native playback and camera. Cover compatible device management, WARP/software
rejection, incompatible resources, actual delayed completion, two consumers,
resize/replacement/device loss, native audio, overlapping seeks, permissions,
hotplug and owned JPEG capture. Record the required Windows packaging and native
keyboard/UI Automation evidence separately from implementation.

## Linux agent

Introduce `FerroUI.Media.Video.Linux`, `FerroUI.Media.Camera.Linux` and a shared
helper only where useful. Use system GStreamer/plugins for playback, with native
audio and timing. Negotiate DMA-BUF memory, format, plane offsets/strides and DRM
modifiers against what the **active renderer** admits. An FD alone is not proof
of compatible storage or synchronization. Own native buffers and FDs through
actual GPU retirement.

Investigate the repository's default Skia Vulkan path as well as EGL/Ganesh.
The existing Vulkan and other legacy snapshot importers have no new direct
acquisition implementation. A successful GL-only experiment must not be reported
as completion of a different default renderer profile. Do not insert CPU
`videoconvert`/download/re-upload stages to conceal negotiation failures.

Camera uses portal-authorized PipeWire, plus an explicit direct V4L2 host profile.
Direct V4L2 must not silently bypass a denied portal request. Discovery remains
non-prompting; native permissions and formats can remain unknown until consent.
Ensure cancellation closes portal requests/sessions, late streams and native
buffer callbacks without UI waits. Capture output and optional controls must be
reported as actually supported by the chosen profile.

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
`native-metal-generation-regression.log`, `native-browser-regression-smoke.log`
and `apple-ios14-check.log`. Re-run applicable gates on the receiving host.

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

- Windows and Linux native providers are unimplemented at this checkpoint.
  Android now has native playback/CameraX/external GLES integration and a
  separate active-media/camera run on a 16 KiB kernel.
- Native QR engines are unimplemented. The optional Rust adapter does not expose
  complete payload bytes, and its internal input-copy behavior still needs
  profiling. Do not infer zero copying from its borrowed input interface.
- `FerroUI.Media.Camera.Controls` now provides CameraPreview with explicit
  start, owned-session teardown and weak caller-session attachment. Complete
  markup/source APIs, native 3D texture-reuse examples and automatic device-loss
  recovery remain unfinished.
- Apple source orientation/crop parity, physical capture/denial/hotplug, minimum
  OS linking/runtime and the broader browser/device matrix need
  separate evidence. Browser native frame output currently requires a page-owned
  single-threaded WebGL2 context, frame callbacks and matching decoded/upload
  dimensions; it rejects CPU-backed uploads. Worker media bridges are deferred.
- All per-profile packaging, native keyboard/accessibility and frozen hardware/
  workload/performance acceptance gates remain open where not explicitly tested.

Return implementation commits, exact build/run commands, observed native
capabilities, counters and unresolved failures. Keep implementation status,
simulated tests, hardware evidence and performance acceptance distinct.
