# macOS CGL media profile

**Status: experimental; the repeated-reset native-memory gate fails on the current Apple M4/macOS 27.0.1 host.**

The opt-in macOS media route uses native CGL/OpenGL and Skia Ganesh. Metal with
Skia Graphite remains the default profile, with separate
[verification](MACOS-VERIFICATION.md). These are separate build configurations;
this work does not add another rendering engine or a Vulkan/ANGLE route.

## Selecting the profile

```sh
RUSTFLAGS='--cfg ferro_skia_apple_ganesh' \
CARGO_TARGET_DIR=/path/to/private/cgl-target \
cargo run -p portable-media -- /path/to/video.mp4 --opengl --renderer
```

The existing Apple Ganesh configuration selects Skia's Ganesh OpenGL/Metal build
instead of Graphite. The media profile verified here explicitly selects CGL with
`--opengl`; Metal/Ganesh media is not claimed. Build the default Metal/Graphite
profile without this compiler configuration. A default Graphite sample rejects
`--opengl` early with build guidance. No native build cache is committed.

The base Media crate, Video and Camera remain independent. The Apple storage
crate keeps CGL view ownership behind its explicit `opengl` feature; its combined
presentation importer remains behind `controls`. The base Apple storage package
without these features has no visual-control dependency. The same AVFoundation video
and camera providers retain CoreVideo/IOSurface storage for either selected
renderer. No additional decode or camera acquisition is introduced.

## Ownership and presentation

- CGL imports the retained IOSurface as a rectangle texture. The composition
  importer borrows that texture directly into Ganesh instead of copying it to a
  snapshot. Color conversion, when needed, stays in the sampling shader.
- `FrameLease::with_keep_alive` associates an API view with one consumer's lease,
  preserving source identity, validity and the original storage budget. A
  deduplicated submission retains every distinct API view of that allocation.
- Each submitted reader uses actual GL sync objects, flushed on its owning
  context and polled with zero timeout. Submission is not completion. Native
  alias deletion and source accounting remain deferred until final view/GPU
  retirement. Unknown completion is quarantined.
- Reset retires the platform's CGL context and creates a replacement with a new
  context identity and globally distinct media generation. Queued producers and
  importers from the old generation are rejected; submitted reads still retire
  on their original native queue before its owners release it.
- `CglFrameView` allows renderer clients to sample the same leased camera/video
  allocation. The sample projects this rectangle texture with a desktop GL
  shader and presents its transparent output as a leased GL texture.
- Admission requires a known compatible device, format, generation and native
  fence support. The CGL route does not declare a CPU preview fallback.

The ordinary composition path preserves clipping, transforms, opacity and
sibling ordering. Native CGL device identity now queries the matched renderer's
registry ID instead of always using renderer index zero. Multi-GPU/Intel runtime
acceptance still requires that hardware.

## Failures found during verification

Earlier failures remain in the evidence directory, outside Git:

```text
<workspace>/build-cache/ferroui-portable-media-evidence/macos-cgl-2026-11-02/
```

`pixels-initial.log` caught an untagged native presentation IOSurface: projected
blue differed from the original expected SDR values. The presented IOSurface is
now explicitly tagged with the serialized sRGB color space. Pixel tolerances
remain unchanged.

`full-run-1/pixels-0.log` passed seek/recovery pixels but failed final retirement.
An offscreen Ganesh raster snapshot could submit internally without closing the
framework's tracked frame submission; the last read remained charged while the
window was idle. The raster-snapshot path now closes that submission and uses
the actual native completion handler. `pixels-submission-fixed.log` records the
same case passing with zero retained frames.

`full-run-2/native-memory-soak.log` exceeded the predeclared 64 MiB footprint
growth limit after 73 completed cycles, with zero tracked frames/reads/native
player owners and flat live Rust allocations. CGL jobs lacked an autorelease
pool covering the direct GL calls made from Rust. `full-run-3/pixels-0.log`
retains a failed context-token pool experiment: graphics-context tokens do not
have a guaranteed stack disposal order, so they cannot own thread-local pools.
The synchronous server-compositor frame now supplies one lexical native scope,
including queued jobs, snapshots and retirement. A native Objective-C ownership
regression checks nested calls and unwinding. This bounds autoreleases, but the
subsequent `full-run-4` memory run still fails: the native growth is not fixed by
an autorelease pool.


The failure at the checkpoint after cycle 73 in `full-run-4` reports footprint
261,342,480 bytes against the cycle-10 baseline 192,709,856: **+68,632,624 bytes**,
exceeding the unchanged 67,108,864-byte limit. The final successful retirement
checkpoint has zero tracked frames/reads/quarantine/player owners and only
64 additional live Rust bytes. The failed profile is not relabeled passing.

Attribution evidence:

| Workload | Completed cycles | Footprint change after cycle 10 | Live Rust change |
| --- | ---: | ---: | ---: |
| Source only | 100 | -1,359,872 bytes | 0 |
| Renderer without resets | 100 | +2,195,456 bytes | 0 |
| Native context replacement plus full workload | 79 before next checkpoint fails | +56,246,296 bytes at cycle 79 | 0 |

`heap-root-pool.log` contains 1,188 `GLRRenderPipelineState` instances and
associated AGX shader variants. `pipeline-attribution-owner-tree.log` traces a
retained pipeline through a CFDictionary owned by the Apple GL driver's
`GLDDeviceRec`. This locates observed retention; it does not prove every cause of
native growth. Further Skia/driver investigation or another driver/OS profile is
needed. Disabling the documented GL build cache succeeds and reads back disabled
but still fails the same gate (`attribute-build-cache-disabled.log`). Actual
native context replacement also fails the gate (`native-context-memory-300.log`),
while opt-in native lifetime logs show old bridge contexts being destroyed.
A resource-reclamation experiment faulted inside the native driver and was
removed; no cache override or reclamation workaround is shipped.

## Passing CGL checks

`remaining-checks/report.json` records **36 passing requested stages**, with the
positive native-memory gate explicitly listed as omitted and `full_run=false`.
Its frozen CGL executable SHA-256 is:

```text
ea46cf119110fd700d8f37238d67f3356e6b4dfddf76ddb924a7321c0ac4c4a2
```

This is partial evidence, independent of the failed long runs above. It covers
all 16 raw/composed/own-window geometry cases, paused seek/recovery and resize,
zero preview/copy counters and final retirement, native input/accessibility,
the deliberate retained-memory negative, 137 shared media/OpenGL unit tests,
seven native CGL tests, the native render-scope ownership test, the Apple camera
delegate lifetime fixture and native Vision fixture. Nineteen frame-contract
tests also pass (`frame-contracts-final.log`), including retention of distinct
API views in a deduplicated GPU submission. The camera fixture does not acquire
hardware.

The final keyboard harness explicitly activates its own app/window and waits
for AppKit's key-window state before dispatch. The precondition is preserved;
input is not sent to an inactive window. The final CGL transport and orientation-7
crop/seek/recovery case also pass in `final-input/report.json`, using executable
`dfa4031443f1f218a3b9a8e0908f94c571a6fb0fff142db3126b0f1ee9271f4d`. Earlier Metal replays stopped at that
precondition; those failures are retained in `metal-final` and
`metal-final-serial`, separate from the final successful replay.

Default CGL playback, recovery and EOF restart pass. A development app bundle is
ad-hoc signed with hardened runtime and camera/network entitlements, passes
strict signature verification, playback and camera discovery. Discovery sees no
devices, acquires nothing and does not prompt. Its default executable has no
ScreenCaptureKit import; its recorded ARM64 deployment floor is macOS 11.0.
Developer ID/notarization and minimum-OS runtime are separate gates.

## Replay and limits

```sh
python3 tests/media/verify-macos.py --backend cgl \
  --output /path/outside/repository/new-evidence-directory \
  --target-dir /path/to/private/cgl-target --cycles 300
```

The runner selects the Ganesh compiler configuration and CGL mode, freezes the
executable hash, retains failures and records source hashes. `--skip-memory`
explicitly runs a diagnostic subset: its report lists the omitted positive
memory gate and emits `MACOS_VERIFICATION_PARTIAL_PASS` rather than full success.
The retained-memory negative control still runs. A subset cannot establish
profile acceptance. It covers all 16
orientation/crop fixtures through raw output, composition and actual own-window
pixels; resize; paused seek/recovery; native accessibility/input; lifecycle and
negative controls; shared contracts; native CGL ownership; development packaging
and deployment metadata. The default diagnostic-capture availability requirement
remains macOS 14.4; production media does not import ScreenCaptureKit.

The lifecycle limits are fixed before warm-up: 64 MiB footprint growth and 64
additional Mach ports after ten warm-up cycles, using the synthetic 160x90
AVC/AAC fixture, 220x220 projection and alternating 800x500/940x620 window with two
resets per cycle. Source and renderer frame/read/quarantine counters must reach
zero at each retirement checkpoint. The diagnostic Metal allocation counter is
also recorded and bounded, but it is not a measurement of all CGL driver/GPU
memory. Process footprint and actual storage ownership remain independent
checks. This low-resolution lifecycle workload is not full performance acceptance.

Physical camera permission/denial, removal/Continuity, independently decoded
photos and camera+QR recovery remain camera-host gates. Audible output/AV sync,
full VoiceOver workflow, abrupt driver loss, older macOS and Intel/multi-GPU
runtime, Developer ID/notarization and full-resolution performance remain
separate release evidence. A passing CGL run does not close these gates or the
other platforms' profiles.

## Default Metal regression

`metal-focus-final/report.json` records all **37 passing stages**, including
300 lifecycle cycles / 600 resets and both expected negative controls, after
the shared lifetime and native frame-scope changes. Its frozen executable is
`846941ac0c95f25b116d0660408efc1ad4a47c727ebb47ba000bef59032a2a65`. Footprint changes by +5,881,856 bytes from cycle 10 to
300, Metal allocation changes by zero and Mach ports decrease by 14; all
retirement checkpoints have zero tracked frame/read/quarantine/player owners.
These results preserve the default Metal profile's measured gates independently
of the failed CGL reset-memory profile.
