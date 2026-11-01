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

## Memory attribution, cancellation races and automated verification

The opt-in sample `memory-audit` feature wraps the unchanged Rust `System`
allocator with allocation-free atomic counters for requested live bytes/blocks,
logical peak bytes and cumulative requested growth. It does not capture native
allocations, allocator padding, stack/inline storage, or transient duplication
inside `realloc`; concurrent snapshots are not an atomic heap census. Its
alignment/zeroing/grow/shrink test passes. The implementation follows the
[GlobalAlloc contract](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html),
including avoiding allocation, logging, locks and unwinding inside the hooks.

The native fixture additionally reads the default process heap's allocated and
committed bytes through dynamically resolved
[HeapSummary](https://learn.microsoft.com/en-us/windows/win32/api/heapapi/nf-heapapi-heapsummary),
and classifies committed virtual regions with `VirtualQuery`. Unavailable heap
statistics use `u64::MAX`. Only the stable default heap is summarized; arbitrary
native heaps are not enumerated/dereferenced while workers could destroy them.
These figures overlap Rust allocations and must not be added together.

Six separate processes each completed 20 cycles: the full recovery workload
with both values of quiet output × PNG processing, then capture-only and UI
compositor-reset workloads without capture. All retirement checkpoints remained
zero. The table shows final minus cycle-5 values, in bytes:

| Workload | Rust live | Default heap allocated | Default heap committed |
| --- | ---: | ---: | ---: |
| Verbose, PNG | 0 | 1,945,033 | 6,701,056 |
| Verbose, raw snapshot | -80 | 1,935,715 | 5,918,720 |
| Quiet, PNG | 80 | 1,905,882 | 6,864,896 |
| Quiet, raw snapshot | 0 | 1,839,395 | 5,693,440 |
| Capture only | 128 | 929,889 | 1,556,480 |
| UI compositor resets, no capture | 128 | 1,087,418 | 2,637,824 |

The full workloads kept live Rust allocations near 2.96 MB. Removing PNG reduced
cumulative requested Rust allocation growth over 20 cycles from about 3.52 GB
to 2.34 GB, but did not remove native heap growth. Quiet output did not remove
it either. Both capture-only and reset-only workloads retained native heap
growth. This localizes the observed growth outside tracked Rust objects and
shows some additional committed heap capacity; it does not identify native
allocation call sites or prove cache retention rather than leaks. A process-only
Xperf heap-stack attachment was attempted without registry changes and returned
`Access is denied (0x5)` under the current non-elevated token. No trace session
was started. Further native call-site attribution remains open.

`--quiet-diagnostics` suppresses routine sample prints while retaining errors
and stress/memory measurements. `--no-png-diagnostics` reads the existing Skia
raster composition snapshot directly instead of encoding/decoding/saving PNGs.
All the same pixel assertions still run, and both snapshot/read operations are
counted as Diagnostic transfers. Preview source textures are never CPU-mapped.
The default build uses no audit allocator and no extra Skia diagnostic dependency.

The new `--lifecycle-stress-cycles=N --synthetic-camera` mode uses real native
work, not only mock or unpolled futures. Each cycle:

- Waits for native video opening to begin before clearing its source, then
  repeats with an in-flight source replacement and observes Cancelled.
- Overlaps twelve polled seeks, verifies cancellation and the newest 700 ms
  result, then replaces a source during a seek. An old frame lease survives the
  replacement; the new mailbox must publish a newer generation.
- Closes an actual secondary camera window during native opening, observing
  cancellation and resource retirement while the owner window stays alive.
- Starts a real native photo job, resets the compositor and closes its window
  on context creation before queued recovery can reacquire the camera. Photo
  delivery must be Cancelled, the session stopped and factory/activation counts
  unchanged. All frame/read/quarantine/native resources retire to zero.

The file-level MP4 suite now runs the actual file inspector through every
truncation, exact/over-limit 4,096-box and 8 MiB boundaries, malformed extended
headers, cancellation before scanning and deterministically after metadata I/O,
and a sparse extended `mdat` beyond 4 GiB. Exclusive reopen verifies exception
paths close file handles. Sparse marking is mandatory before that large seek;
no dense multi-gigabyte file is created. This tests metadata seeking, not native
decoding of a multi-gigabyte movie.

New rational edge cases exposed an admission defect: the former `1e-6` rounding
tolerance accepted tiny fractional aperture widths/origins in the integral-only
profile. Extents and final origins now use exact, overflow-bounded integer
rational arithmetic. Fractions are rejected even when visually negligible;
fractional offsets that mathematically resolve an odd-sized aperture to an
integral origin remain valid. The failing pre-fix test is preserved, and all
three native geometry/D3D11/ANGLE tests pass after the correction. The three
native-source preparation contracts also pass, covering bounded filesystem
workers, cancellation during a write and removal of late output.

The complete Windows verification command (PowerShell 7, Rust/Node on PATH) is:

```powershell
./tests/media/verify-windows.ps1 -TargetDirectory F:\build-cache\ferroui-portable-media-windows
```

It builds and freezes an audited executable, runs the six memory workloads,
native/allocator/source contracts, all 16 video geometry/seek/recovery cases,
cancellation races and a camera soak, then builds/tests default playback and
fixture rejection. Each child has a timeout, streamed stdout/stderr logs,
required pass markers and checked exit status. Soaks require every ordered
zero-resource checkpoint. The runner's own positive/negative controls prove
handling of expected rejection, missing evidence, wrong exits and timeouts.
Arguments/output paths containing spaces are supported. JSON and Markdown
reports retain commands, timings, binary SHA-256 and changed-source hashes.
Failure preserves evidence and exits nonzero; a fresh output directory prevents
mixing old pass markers into a new run. The runner restores environment settings.

Optional arguments include `-MemoryCycles`, `-RaceCycles`, `-StressCycles`,
`-EndedRestartRuns` (five by default),
`-Section Memory`, `-Section HarnessSelfTest`, `-OutputDirectory`, `-Node`, and
`-IncludeWsl -WslTargetDirectory /home/danw/build-cache/ferroui-portable-media`.
WSL inclusion builds the native Linux sample and runs shared video-control
contracts; it does not claim DMA-BUF pixels.
Use the WSL cache path appropriate to the host. The default workload counts are
20; memory workloads require at least six cycles to retain five warm-up samples.

Evidence beside the worktree includes `windows-memory-comparison-final/report.json`
and its per-workload logs, `windows-heap-trace-start.log`,
`windows-lifecycle-stress-pilot-2.log`, `windows-mp4-fractional-before.log`,
`windows-mp4-file-tests-exact.log`, `windows-source-preparation-tests.log`,
`windows-allocation-audit-tests.log` and `windows-runner self test final/report.md`.
The first end-to-end run correctly stopped at an intermittent default-build EOF
restart failure and preserved `windows verification complete/report.json`.
Native Play completed, but the source/renderer mailboxes remained at their final
pre-restart frame until the smoke timed out. `MediaPlayerElement` gave delayed
native state changes a bounded settling allowance while Paused, but immediately
stopped polling if the old state was still Ended. Play could restart the timer,
then lose it before the asynchronous native command reached Playing.

The shared control now gives Ended the same bounded allowance, preserving both
fresh-frame polling and the settled-EOF shutdown. A deterministic delayed-state
regression fails before the fix; all nine Windows video-control tests and all
nine corresponding WSL tests pass afterward. The final runner includes that test
and five fresh native EOF restart processes. `windows-ended-polling-before.log` and
`windows-ended-polling-fixed.log` preserve this result.

The final end-to-end rerun passes all 45 stages in approximately 8 min 45 s,
including all 16 video cases, three race cycles (36 overlapping seeks), six
full camera soak cycles / 36 resets, all five native EOF restarts, default
recovery/rejection and both WSL steps. The native suite passes three tests,
allocator suite one, source preparation three, and Windows/WSL video controls
nine each. Every race/soak retirement checkpoint is zero. Earlier six-cycle
races and ten-cycle soak also passed before the EOF failure was isolated; those
partial-run successes do not turn the original failed report into a pass.

```powershell
./tests/media/verify-windows.ps1 -MemoryCycles 6 -StressCycles 6 -RaceCycles 3 `
  -EndedRestartRuns 5 -IncludeWsl `
  -WslTargetDirectory /home/danw/build-cache/ferroui-portable-media `
  -TargetDirectory F:\build-cache\ferroui-portable-media-windows `
  -OutputDirectory '..\windows verification final'
```

The final reports/logs are in `windows verification final/report.json` and
`report.md` beside the worktree. That successful run uses six-cycle memory
smokes; the separate six × 20-cycle attribution evidence above remains the
longer comparison. `windows-runner strict self test/report.md` also records
successful harness checks under PowerShell StrictMode. Physical/native driver
acceptance and native heap call-site attribution remain open as described above.
