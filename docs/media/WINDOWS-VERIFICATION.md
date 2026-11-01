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

## Local MP4 geometry and sustained lifecycle follow-up — 2026-11-01

The old playback smoke passed rotated/reflected fixtures because it checked
progress and transfer counters without independently checking their pixels.
Actual presentation omitted reflection/rotation and gave the cropped fixture a
144×80 natural size instead of 80×144. Stronger boundary probes then showed the
native RGB processor centring/resizing a clean aperture while losing its
off-centre offsets. The failing pixel and boundary logs are preserved.

The bridge now reads local ISO-BMFF metadata on its existing worker, skips media
payload by seeking, bounds top-level/nested scanning to 4,096 boxes per scan and
`moov` allocation to 8 MiB, and admits one unambiguous video track/sample
description, integral clean aperture and square pixels. It recognizes all eight
orthogonal track matrices. Unsupported/malformed geometry fails explicitly.
Before the first read it configures the Source Reader's video processor to
return full coded pixels, with no native rotation, reflection or crop. The
shared GPU layout applies the exact immutable clean aperture and transform;
natural size is the rotated clean extent. No preview CPU pixel operation or
framework GPU copy was introduced. Other containers and remote URLs do not
inherit this local-MP4 matrix claim.

The fixture generator changes only track-matrix bytes in checked-in encoded
fixtures. The independent acceptance model uses known quadrant colours and
source coordinates, not provider-reported transforms. All 16 combinations
(eight orientations × full/off-centre crop) pass raw projected RGBA diagnostics,
composition snapshots and window-only presentation capture, initially and after
resize. Dense crop-boundary probes allow one decoded source pixel plus capture
rounding; wrong-orientation and wrong-offset negative controls fail. All 16 also
pass composition/presentation checks after seeking paused playback to 500 ms and
after compositor recovery, preserving pause, position and geometry. Old leases
are held across recovery, then source/renderer credits, reads, quarantine and
native workers retire to zero. CPU preview attempts/bytes and framework GPU
copies remain zero; native RGB conversion and explicit diagnostics are counted.

```powershell
cargo build --locked -p portable-media --features synthetic-camera --target x86_64-pc-windows-msvc
node tests/media/make-windows-geometry-fixtures.mjs ..\windows-geometry-variants
$app = 'F:\build-cache\ferroui-portable-media-windows\x86_64-pc-windows-msvc\debug\portable-media.exe'
$env:FERROUI_MEDIA_EVIDENCE = 'C:\Users\Dan\.codex\worktrees\portable-media-linux\video-geometry-evidence'
foreach ($crop in @($false, $true)) {
    foreach ($orientation in 0..7) {
        $suffix = if ($crop) { '-crop' } else { '' }
        $arguments = @("..\windows-geometry-variants\orientation-$orientation$suffix.mp4", '--renderer', '--video-pixel-smoke', '--video-lifecycle-pixels', "--orientation=$orientation")
        if ($crop) { $arguments += '--crop-smoke' }
        & $app @arguments
        if ($LASTEXITCODE -ne 0) { throw 'Video geometry failed' }
    }
}
cargo test --locked -p ferroui-media-windows --features controls,native-frame-tests,synthetic-camera --target x86_64-pc-windows-msvc --lib -- --nocapture
```

Use a fresh fixture output directory: the generator refuses to overwrite files.
The `native-pixel-tests` sample feature can run video checks without enabling a
synthetic camera; `synthetic-camera` includes it and adds native retirement
counters. Parser tests cover both track-header versions, extended box sizes,
all truncations and malformed/ambiguous/nonorthogonal/fractional metadata. All
three native tests pass, including independent D3D11 pixels and ANGLE aliasing.

The camera stress harness repeats the complete capture/photo/disconnect suite
and six camera-control compositor resets per cycle in one process, alternating
window sizes. It checks zero frame credits, GPU reads, quarantine, native
textures/sources/workers after every cycle, and records process private/working
set bytes, DXGI local-memory usage and process handle count. After five warm-up
cycles it guards against another 128 MiB of private or local GPU memory, or 64
handles. Those are diagnostic growth limits, not product performance targets.

The first pilot found substantial device retention despite zero frame budgets:
private bytes grew from 286,584,832 at cycle 1 to 1,191,559,168 at cycle 6, local
GPU bytes from 179,499,008 to 828,542,976, and handles from 1,426 to 5,297.
`LockBound` previously leaked render-bound values dropped off the owning thread;
an interop handle consequently retained each old backend/device. Destruction now
queues to and wakes the owning render thread. Regression checks pass 561 Windows
composition tests, nine Windows context-retirement manager tests and 15 WSL
render-thread tests, including owner-thread destructor probes. A fixed eight-cycle
pilot (48 resets) ended at 161,030,144 private bytes, 74,141,696 local GPU bytes
and 777 handles.

A longer run independently exposed one remaining native texture at idle, with
zero frame/alias wrappers, fixture source bindings, active photos and photo
quarantine. Disposing the context released it. D3D11 defers destruction; the
bridge now calls Flush after worker resource cleanup and after frame release
when its source is closed/gone. The latter checks closure after deletion to
avoid racing the worker's last Flush. Streaming retirement adds no extra Flush.
The zero-resource assertion remains mandatory; failure diagnostics/reset never
turn a failed run into a pass. Flush only submits deferred work and does not
establish completion or release quarantine. See Microsoft's
[Flush contract](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-flush)
and [Source Reader transform configuration](https://learn.microsoft.com/en-us/windows/win32/api/mfreadwrite/nf-mfreadwrite-imfsourcereaderex-gettransformforstream).

```powershell
$env:FERROUI_MEDIA_EVIDENCE = 'C:\Users\Dan\.codex\worktrees\portable-media-linux\camera-stress-evidence'
& $app --camera --synthetic-camera --camera-smoke --camera-recovery-smoke --renderer --camera-stress-cycles=120
```

The final run passes 120 cycles / 720 resets in 1,231,338 ms (20 min 31 s).
Every checkpoint has zero frame credits, GPU reads, quarantine and native
textures/sources/workers; preview CPU attempts/bytes remain zero. Resource
measurements are bytes except handles; maxima are post-cycle samples, not
instantaneous peaks:

| Measurement | Cycle 5 baseline | Cycle 120 | Maximum post-cycle sample |
| --- | ---: | ---: | ---: |
| Private bytes | 160,002,048 | 178,118,656 | 190,062,592 |
| Working set bytes | 163,500,032 | 193,843,200 | 195,149,824 |
| DXGI local GPU bytes | 74,797,056 | 65,425,408 | 76,763,136 |
| Process handles | 780 | 774 | 788 |

Private memory grew by about 17.3 MiB at the final checkpoint and peaked 28.7
MiB above the warmed baseline. This needs heap/allocator/SDK profiling before
any leak-free or frozen performance claim. Process memory includes opaque
SDK/driver caches and allocator retention. The soak passes its diagnostic
growth guards, which do not resolve that attribution. Default Windows playback
also passes resource loading, renderer sharing, paused recovery and end-of-stream
restart with final retained frames zero; default builds reject synthetic flags.
The video-only pixel feature passes a cropped/reflected seek/recovery run.
The final native WSL sample build passes. Physical cameras/privacy/hotplug,
abrupt driver removal, other codec/colour profiles and a DMA-BUF-capable Linux
host remain necessary.

Logs/artifacts are outside Git beside this worktree:
`windows-video-pixels-before.log`, `windows-video-pixels-boundary-tight.log`,
`windows-video-pixels-crop-normalization-before.log`,
`windows-video-pixels-{0..7}[-crop].log`,
`windows-video-lifecycle-{0..7}[-crop].log`, `video-geometry-evidence/*.png`,
`windows-video-geometry-native-tests-final.log`,
`windows-camera-stress-pilot.log`, `windows-camera-stress-fixed-pilot.log`,
`windows-camera-stress-retirement-failure.log`,
`windows-camera-native-handles-diagnostic.log`,
`windows-camera-stress-idle-flush.log`, `windows-camera-stress-binary-hash.log`,
`camera-stress-evidence/*.png`, `windows-composition-owner-drop-tests.log`,
`windows-context-retirement-manager-tests.log`, `linux-owner-drop-tests.log`,
`windows-geometry-default-build.log`, `windows-geometry-default-recovery.log`,
`windows-geometry-default-ended.log`, `windows-geometry-default-rejection.log`,
`windows-video-pixel-only-build.log`, `windows-video-pixel-only.log`
and `linux-geometry-final-build.log`.
