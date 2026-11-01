# Windows native verification — 2026-11-01

Starting revision: `6aac17e56e81e3ecd2b14113fc02ea0979be0f59` from
`origin/codex/portable-media-framework`. Platform work is isolated on
`codex/portable-media-windows`. All 19 existing media-branch commits already use
`Wiesław Šoltés <wieslawsoltes@users.noreply.github.com>` as committer; no history
rewrite was needed. Authors remain unchanged.

## Environment and fixes

Windows 11 Pro build 26200, NVIDIA GeForce GTX 1660 Ti, driver 32.0.16.1088,
Rust/Cargo 1.93.0, MSVC 14.50.35717 and Windows SDK 10.0.26100.0. ANGLE is the
repository's statically linked mozangle 0.7.1. Native compilation is enabled;
`FERROUI_NATIVE_MEDIA_CHECK_ONLY` is unset.

The first full MSVC build linked successfully. Native fixture playback then
exposed an actual format mismatch: MF RGB32 produced XRGB storage that the
bundled ANGLE cannot import. Request ARGB32/BGRA instead and reject XRGB at
admission. No CPU conversion or preview fallback was added.

Controlled context recovery exposed a paused seek/Stop race: up to three
read-ahead frames could become GPU-ready before the first frame was published.
The worker subsequently replaced the paused frame with later queued frames.
Publish the first sought frame once and retain future frames for resume. An
independent native test exercises the queued GPU-ready frames and resume.

Keyboard verification exposed Play at Ended hanging against MF's EOF state.
Restart seekable playback in a new publication epoch, wait for that epoch's
first native frame, and prevent an old EOS iteration from overwriting a newer
seek/play command. The `--ended-restart-smoke` harness requires a newer source
sequence, a near-start timestamp and an advancing Playing clock before continuing
the normal pause/seek/Stop checks; an accepted Play command alone is insufficient.

Count the requested MF RGB-processing boundary once per admitted sample, rather
than again on a failed admission/retry. This remains an accounting boundary,
not an audit of native MFT/codec/driver internals or proof of hardware decoding.

## Evidence

- Two opt-in native tests pass. Independent D3D11 GPU clears produce four
  quadrants in texture-array slice 1. Actual D3D readiness and ANGLE image/FBO
  diagnostic reads preserve the pixels. Native sample/array retention,
  incompatible device/size/bind flags, unsupported XRGB, WARP rejection and
  paused read-ahead/resume pass. These diagnostic/photo reads are test-only;
  production preview remains unmapped.
- The shared media, camera, GL ownership and video suites pass **40 tests**
  (5 + 7 + 3 + 25), including queued cleanup and unknown-completion accounting.
- Native AVC/AAC resource playback, pause, achieved 500 ms seek, resume,
  Stop/frame zero and final allocation retirement pass. The same source feeds
  the normal view and the client-owned GLES projected renderer. Controlled
  context replacement preserves paused intent/position, creates a new source
  generation, resumes rendering, and retires both source and renderer storage.
- Ended/restart plus renderer reuse passes the strict near-start/new-frame gate
  and then pause/seek/Stop/retirement. Final resource, combined renderer/recovery
  and repeated recovery runs pass. Admitted production preview counters show
  **zero CPU attempts/bytes and zero GPU copies**. Required native RGB processing
  is counted separately.
- Camera discovery returns zero devices with NotDetermined permission before
  and after and `acquired=false`. There is no physical camera on this profile.
- Actual Windows UI Automation exposes Play/Pause, Stop, Playback position,
  Volume, Mute and the playback-status text. Tab and Shift+Tab move focus between
  transport buttons; Space activates Stop (Paused at 0.0 s) and Play (Playing
  with an advancing position). Visual inspection shows the four source
  quadrants and projected renderer. Full screen-reader acceptance is separate.
- A copy of the executable runs the resource smoke from a separate directory.
  PE dependency inspection shows system MF/D3D/UI DLLs and the MSVC/UCRT runtime;
  ANGLE does not require companion EGL/GLES DLLs. This host already has the
  Visual C++ runtime installed; this is not a clean-machine installer test.

## Reproduction

From the worktree root in PowerShell:

```powershell
$env:PATH = "C:\Users\Dan\.cargo\bin;" + $env:PATH
$env:CARGO_TARGET_DIR = 'F:\build-cache\ferroui-portable-media-windows'
cargo build --locked -p portable-media --target x86_64-pc-windows-msvc
cargo test --locked -p ferroui-media-windows --features native-frame-tests --target x86_64-pc-windows-msvc --lib -- --nocapture
cargo test --locked -p ferroui-media -p ferroui-media-video -p ferroui-media-camera -p ferroui-media-opengl --features ferroui-media-opengl/controls --target x86_64-pc-windows-msvc --lib
$app = Join-Path $env:CARGO_TARGET_DIR 'x86_64-pc-windows-msvc\debug\portable-media.exe'
& $app tests/media/fixtures/avc-aac.mp4 --resource --smoke
& $app tests/media/fixtures/avc-aac.mp4 --resource --smoke --renderer --device-recovery-smoke
& $app tests/media/fixtures/avc-aac.mp4 --resource --smoke --renderer --ended-restart-smoke
& $app --camera-discovery
```

Raw logs are outside Git beside the Windows worktree in
`C:\Users\Dan\.codex\worktrees\native-desktop-platforms\`:
`windows-build-final.log`, `windows-native-frames-final.log`,
`windows-media-contracts.log`, `windows-final-resource.log`,
`windows-final-renderer-recovery.log`, `windows-final-ended-restart.log`,
`windows-final-recovery-repeat.log`, `windows-camera-discovery.log`,
`windows-packaged-playback.log`, `windows-pe-dependencies.log`,
`windows-environment.json` and `windows-accessibility.json`.
The original format and recovery failures remain in the earlier logs.

## Remaining acceptance

Physical camera preview, privacy denial/cancellation, hotplug and independently
decoded JPEG capture require a device. Wider codecs, geometry/color profiles,
abrupt driver loss, independently measured native audio/AV sync, full Narrator
coverage, older Windows/clean-machine installation and frozen sustained
performance workloads remain open. WGL/Vulkan direct media routes and desktop
native QR are not established by the exercised ANGLE/GLES profile. Functional
passes here do not declare complete portable-media release acceptance.

## Synthetic camera follow-up — 2026-11-01

The optional `synthetic-camera` feature now supplies an **in-process Media
Foundation camera source**, using the same provider, asynchronous Source Reader,
DXGI admission, ANGLE aliases, public camera sessions and JPEG worker as capture.
This is original fixture code using the Windows SDK; no third-party camera
software or system device is installed. It is not a globally registered Windows
virtual camera and makes no claim about OS privacy, Frame Server or hardware.
The [MF source object model](https://learn.microsoft.com/en-us/windows/win32/medfound/media-source-object-model)
describes the interfaces used; an OS-visible virtual camera additionally needs
registration through the [virtual-camera API](https://learn.microsoft.com/en-us/windows/win32/api/mfvirtualcamera/nf-mfvirtualcamera-mfcreatevirtualcamera).

Discovery exposes explicitly labeled synthetic selections only when the caller
chooses `WindowsCameraProvider::synthetic`. Neither source creation nor worker
acquisition occurs during discovery. The fixture generates 320×240 SDR BGRA
quadrants using GPU clears on the exact compositor device at 30000/1001 fps.
Every sample has fresh immutable storage. A credit retained on each D3D texture
bounds live fixture textures to 16 across sessions; source/native worker counts
are separately observed. Default builds exclude the fixture. The sample rejects
synthetic-only flags if the feature or Windows target is missing.

The extended native smoke verifies:

- Canceling a start after its native worker has started, before commitment;
  permission remains NotDetermined and native resources retire to zero.
- Two successful capture generations, each feeding normal preview and a new
  projected GLES renderer. The test waits for real renderer output in each.
- Retaining three distinct source leases to saturate the frame budget, then
  releasing them and observing publication resume within the same session.
- Actual fractional negotiated frame rate and zero preview CPU transfers.
  No GPU copy occurs before explicit photo work.
- Independently decoded 320×240 JPEG dimensions and all four expected quadrant
  colours, with PreviewSnapshot provenance and counted Photo transfers.
- Stop canceling delivery after a native photo job starts, and completed JPEG
  bytes remaining independently usable after both native and GPU teardown.
- Zero source/renderer allocations, GPU reads, quarantined reads, native source
  objects, fixture textures and native workers after each successful stop.
- A third source injecting `MF_E_VIDEO_RECORDING_DEVICE_INVALIDATED` through MF.
  The real Source Reader callback propagates `DeviceLost`, stops the public
  session, clears its preview and retires native resources to zero.

This added production error classification for removed cameras and D3D device
removed/reset/hung errors. Failed native events are latched for the worker
rather than silently ignored or overwritten by a normal state update.
Fatal read callbacks also use the latched error slot, so a later normal callback
cannot overwrite an error before the worker sees it. Frame polling checks native
failure before waiting for pending imports or credits. Native regression checks
exercise error-then-success callback ordering and callbacks after close. Error
messages include the operation phase. The sample's renderer retirement probe
also now tracks restarted hosts instead of observing only its first budget.

Run the synthetic profile explicitly:

```powershell
cargo build --locked -p portable-media --features synthetic-camera --target x86_64-pc-windows-msvc
& $app --camera --synthetic-camera --camera-smoke --renderer
& $app --camera-discovery --synthetic-camera
cargo test --locked -p ferroui-media-windows --features controls,native-frame-tests,synthetic-camera --target x86_64-pc-windows-msvc --lib -- --nocapture
```

Logs for this follow-up are beside the combined worktree in
`C:\Users\Dan\.codex\worktrees\portable-media-linux\`:
`windows-synthetic-build-final.log`, `windows-synthetic-camera.log`,
`windows-synthetic-camera-repeat.log`, `windows-synthetic-camera-repeat-2.log`,
`windows-synthetic-native-tests.log`,
`windows-camera-default-discovery.log`, `windows-camera-default-regression.log`
and `linux-synthetic-regression-build.log`.

These results establish native provider/camera-session and preview-snapshot
behaviour with controlled GPU inputs. Physical capture-format negotiation,
YUY2/MJPEG/NV12 camera conversion, actual privacy denial, OS device hotplug,
Frame Server registration and abrupt driver-loss recovery still need their own
profiles. Linux's missing WSL DMA-BUF capability is unchanged by this fixture.

## Camera recovery and presentation pixels — 2026-11-01

The opt-in native smoke now additionally exercises the real `CameraPreview`
control, rather than manually restarting only the sample's capture controller.
The `/recovery` MF fixture rotates its GPU quadrant palette on every activation
and adds asymmetric cyan/yellow stripes and a magenta border. Old imagery,
orientation, letterboxing and crop therefore have independently observable
pixel signatures. The ordinary and `/disconnect` fixtures retain their original
four-colour pattern. No OS device is registered.

```powershell
cargo build --locked -p portable-media --features synthetic-camera --target x86_64-pc-windows-msvc
$env:FERROUI_MEDIA_EVIDENCE = 'C:\Users\Dan\.codex\worktrees\portable-media-linux\camera-recovery-evidence'
& $app --camera --synthetic-camera --camera-smoke --camera-recovery-smoke --renderer
cargo test --locked -p ferroui-media-camera-controls --target x86_64-pc-windows-msvc --lib
```

The recovery extension passes six controlled compositor resets:

- Three successful automatic restarts: two during active GPU capture and one
  after native MF disconnection has already stopped the public session. The
  latter exercises the control's separate poll-error suspension path.
- One deliberately failed provider factory, with cleared preview pixels in
  both composition and window captures, followed by successful retry on the
  next context creation.
- Explicit Stop and Detach defeating queued recovery before another native
  source can be acquired. Reattachment alone never starts capture.
- A caller-owned session shared by a second `CameraPreview`. Its factory is
  never invoked; detaching that borrowed control leaves capture publishing.
- A real native photo job started before reset, whose delivery becomes
  Cancelled, followed by independently decoded JPEGs from recovered sessions.
- Holding an old source lease across reset, checking it remains charged and
  rejecting its import through the restored context. Final source/renderer
  allocations, GPU reads, quarantine, MF sources, fixture textures and workers
  all return to zero.

Pixel checks cover three distinct boundaries. An explicit GL diagnostic reads
the actual projected renderer texture, checking the independent perspective
calculation, all four RGBA colours, premultiplied opacity and transparent
corners. A normal composition visual snapshot includes the source controls and
Skia-composited projection over a known background. Finally, window-only
`PrintWindow(PW_CLIENTONLY | PW_RENDERFULLCONTENT)` captures check those same
colours after presentation. These are client-window capture results, not
physical display/scanout colour measurements. No other desktop content is
captured.

The initial window and a resized window pass. Both owned and borrowed controls
pass dense asymmetric pixel probes for Fill, Uniform letterboxing and
UniformToFill cropping at a non-source aspect ratio. Wrong-palette and
wrong-stretch negative controls must fail. Source readback/encoding/decoding,
composition diagnostics, GL probes and window captures are explicitly counted
as Photo or Diagnostic transfers; preview CPU attempts/bytes remain zero.
Raw projection diagnostics belong to the renderer's budget; the final camera
counter line is not an aggregate of both budgets.

A repeated run exposed a verification timing issue: the old five-second
retirement deadline equalled the native worker's five-second MF Flush allowance.
Retirement now allows ten seconds for acknowledgement plus cleanup, rejects
quarantine immediately and reports retained-resource counters on failure.
Synthetic native Flush timeouts also log explicitly. The original timeout log
is preserved; it did not contain enough counters to establish its precise cause.
These functional checks do not establish a frozen teardown/performance target.

Evidence is outside Git beside this worktree: `windows-camera-recovery-final.log`,
`windows-camera-recovery-repeat.log`, `windows-camera-recovery-native-tests.log`,
`windows-camera-recovery-controls-tests.log`,
`windows-camera-recovery-invalid-args.log`,
`windows-camera-recovery-default-build.log`,
`windows-camera-recovery-default-rejection.log`,
`windows-camera-recovery-playback-regression.log`,
`windows-camera-retirement-failure.log` and `camera-recovery-evidence/*.png`.
The native D3D11/ANGLE tests and both camera-control contract tests pass. Default
Windows and WSL sample builds are also checked; Linux's DMA-BUF capability
restriction remains unchanged.

No camera-control production defect was found in these controlled recovery and
pixel cases. Actual privacy/Frame Server behaviour, physical camera unplugging,
abrupt driver removal, other capture formats, wider colour profiles and Linux
DMA-BUF presentation still require suitable devices/hosts.
