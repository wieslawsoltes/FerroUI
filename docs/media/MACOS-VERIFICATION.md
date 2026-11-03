# macOS Metal media verification

This note covers the default **Metal/Skia Graphite** media profile. The later
opt-in CGL/Ganesh addition has separate [verification](MACOS-CGL-VERIFICATION.md). This note
records implementation fixes and measured acceptance on the Apple M4 development
Mac, macOS 27.0.1 (26A434), ARM64, with the macOS 27 SDK. It does not declare all
portable-media release gates complete.

## Native fixes found by the stronger checks

The native `CAMetalLayer` now declares sRGB. Leaving it untagged allowed the
display's device color profile to change presented SDR values relative to the
compositor snapshot. The strict window-pixel probe caught this: a projected blue
sample was `[40, 0, 235]`, outside the existing 30-channel tolerance for the
independently expected `[5, 5, 222]`. Explicit sRGB tagging passes the original
assertions; the tolerances were not widened.

`MediaPlayer::set_options` now emits `OptionsChanged` after native acceptance and
storing the new options. Previously programmatic volume changes left the
transport slider stale until a later playback update. A deterministic idle-state
control test verifies volume/mute synchronization without opening or polling
media, including rejection of an invalid volume. The native keyboard volume
probe also passes after this fix.

The Metal example tracks every live/retiring renderer budget, replacing its
previous first-instance-only budget tracking. Native player-owner counts and
Rust allocation accounting are opt-in development features.

## Graphite dependency ownership

The initial combined resize/reset soak exceeded its predeclared 64 MiB footprint
growth limit at the checkpoint following 153 completed cycles, although frame,
GPU-read, quarantine and native player-owner counters were zero. Source-only,
renderer-lifetime, resize-only and fixed-size reset runs isolated the growth to
combined resizing and context recreation. Rust live allocations stayed nearly
flat while native heap allocations grew.

Retained allocation stacks locate the growth in Graphite's Metal pipeline
creation. In the pinned Skia source, retained vertex/fragment functions, a vertex
descriptor and a color attachment were passed to owning descriptor properties
without balancing the caller's references. The fix adopts those four references
with Skia's existing `sk_cfp` manual-ownership helper. Graphite's other ownership
code requires manual reference counting; enabling ARC for the whole backend is
not the fix.

The root Cargo patch selects the preserved `skia-bindings` 0.153.3 source in
`external/rust-skia-bindings`. macOS Graphite/Metal builds bypass the affected
published archive and apply the strict, idempotent source hotfix. The binding
crate's original MIT license, attribution and VCS metadata are preserved, and
the Skia patch carries its BSD license. See
[FERROUI-PATCH.md](../../external/rust-skia-bindings/FERROUI-PATCH.md) for build
requirements and the downstream publishing boundary. Other target/backend
selections retain the upstream build/download path.

Original failures remain failures. They are retained in `full-run-1` (stale
volume transport) and `full-run-2` (native memory), with earlier capture-coordinate
and untagged-color probes alongside them. Native heap snapshots are
`heap-full-10.log` and `heap-full-60.log`; verified compressed allocation stacks
are `malloc-full-stacks.log.gz`. Evidence is outside Git in:

```text
/Volumes/SSD/build-cache/ferroui-portable-media-evidence/macos-verification-2026-11-02/
```

## Replay and observed results

```sh
python3 tests/media/verify-macos.py \
  --output /path/outside/repository/new-evidence-directory \
  --target-dir /path/to/private/cargo-target \
  --cycles 300
```

Use Xcode command-line tools, Python, Node and Ninja. The source build supports
`SKIA_NINJA_COMMAND`; `SKIA_SOURCE_DIR` can select a prepared pinned source cache.
The runner rejects a caller-managed `SKIA_LIBRARY_SEARCH_PATH` so acceptance
cannot silently import an unreviewed native archive. A fresh source build needs
the pinned native build dependencies available. No native build cache or media
capture is committed.

`full-run-3/report.json` and `report.md` record all **36 passing stages**, including
two expected negative controls. The report records the pre-commit branch revision,
changed-source hashes, native source override and frozen executable SHA-256:

```text
82f9b914f5976c07e43255d87f23d4f657dddcd9dad6eae0d785cb4b4a5eef10
```

A subsequent build also passes without `SKIA_SOURCE_DIR` or a caller-managed
archive, using the crate's ignored prepared source cache and
`SKIA_NINJA_COMMAND`. Its build log confirms the default path bypasses the
affected archive automatically. `default-source-path/report.json` and its logs
record fresh pixel/recovery and lifecycle smoke checks of that build.

Observed checks:

- All eight orthogonal orientations, each with/without an off-centre crop: raw
  completed Metal output, compositor pixels and actual own-window pixels, resize,
  achieved seek, recovered paused presentation and full retirement. Expectations
  are independent of framework layout. Wrong orientation and crop-offset controls
  fail their original assertions. Native window capture uses ScreenCaptureKit's
  current-process-only API; no other desktop/window content is requested.
- Native NSAccessibility exposes three named buttons and two sliders. AX mute
  actions and AppKit keyboard play/pause, seeking, volume and Stop/zero-position
  dispatch reach the real player. Default playback regressions independently
  verify Stop/frame-zero presentation and EOF restart. This is native API/input
  evidence, not a full VoiceOver session.
- 60 shared media/control unit tests, one real Metal ownership/pixel/completion
  test, one Foundation-backed camera delegate lifetime test and one native Vision
  multi-symbol fixture pass. The camera lifetime fixture does not acquire hardware.
- The default build and packaged playback/recovery pass. A development app bundle
  is ad-hoc signed with hardened runtime, camera/network entitlements and the camera
  usage description; strict signature verification and launch pass. The default
  artifact does not import the development ScreenCaptureKit dependency. Its load
  command records macOS 11.0, preserving the ARM64 deployment floor. This is not
  minimum-OS runtime or Developer ID/notarization evidence.
- Packaged camera discovery reports zero devices, unchanged `NotDetermined`
  consent and `acquired=false`. Discovery does not prompt or activate capture.

The diagnostic window-capture feature requires macOS 14.4 and weak-links
ScreenCaptureKit. Production media does not acquire that additional requirement.

## Frozen native-memory workload

The profile is printed before warm-up: the synthetic 160x90 AVC/AAC fixture,
220x220 transparent projected output, one persistent window alternating between
800x500 and 940x620, ten warm-up cycles, two compositor resets per cycle, repeated
source preparation/replacement, superseded seeks and final resource retirement.
Memory runs mute playback. Limits remain 64 MiB footprint growth, 32 MiB Metal
allocation growth and 64 additional Mach ports after warm-up. This is a lifecycle
workload, not the specification's full-resolution performance acceptance.

Final 300-cycle / 600-reset run (about 176 seconds):

| Measurement | Cycle 10 | Cycle 300 | Change |
| --- | ---: | ---: | ---: |
| Process footprint | 210,781,384 | 215,450,824 | +4,669,440 bytes |
| Metal allocated bytes | 44,826,624 | 44,826,624 | 0 |
| Live Rust bytes | 3,169,379 | 3,185,955 | +16,576 bytes |
| Live Rust blocks | 22,345 | 22,340 | -5 |
| Native malloc bytes, including Rust | 29,803,312 | 33,783,712 | +3,980,400 bytes |
| Mach ports | 558 | 547 | -11 |

Every retirement checkpoint reports zero source frames, GPU reads, quarantine,
native player owners and retained renderer frames. There are 300 observed pending
source-preparation cancellations and 1,800 superseded seeks; CPU preview attempts
and bytes remain zero. Positive measurements remain below the original limits.
The retained-memory negative control deliberately holds touched 16 MiB blocks
after warm-up and fails the same footprint gate. Smaller residual native growth
is recorded rather than described as universal leak freedom.

The `source`, `renderer`, `reset`, `resize` and `full` attribution workloads are
available through `--macos-memory-workload=...` together with
`--macos-lifecycle-cycles=...`, `--renderer` and `--features macos-verification`.
Rust allocation counters, native ownership probes and capture instrumentation are
absent from default builds.

## Remaining acceptance

Physical camera denial, device removal/Continuity, native JPEG output and
camera+QR+projection recovery require a camera-equipped host. Actual audible
output/AV synchronization, a full VoiceOver workflow, abrupt driver loss, oldest
supported OS and Intel runtime/driver profiles remain unverified. Developer ID
signing/notarization, clean-machine installation and full-resolution performance
thresholds/workloads remain release gates. Independently published FerroUI crates
must also carry an upstream/published corrected Skia dependency: a workspace
Cargo patch is not inherited by downstream crate consumers.

## Subsequent shared-code regression

The optional CGL work added per-consumer native-view retention, a lexical Apple
render-frame autorelease scope and explicit native-window activation/waiting in
the input harness. `metal-focus-final` in the CGL evidence directory records a
fresh 37-stage Metal replay with 300 cycles / 600 resets passing; see
[MACOS-CGL-VERIFICATION.md](MACOS-CGL-VERIFICATION.md#default-metal-regression).
CGL's separate failing reset-memory gate does not change that Metal evidence.

## Catalog camera frame-rate boundary regression

A physical-camera attempt in ControlCatalog exposed AVFoundation rejecting
`30.0000002` fps for a device whose supported maximum was 30 fps. The previous
native setup converted `1/30` to nanoseconds (`33333333/1000000000`), making
the requested interval slightly shorter than the supported exact minimum.

Format selection now retains the selected range's exact rational duration at
either boundary. Intermediate requested durations are clamped to that range's
native minimum/maximum. Both active duration setters receive this valid value;
negotiated settings continue reporting what the device actually accepts.

The native contract test reproduces the old invalid rounding as a negative
control, verifies exact 30 fps and 30000/1001 boundaries, the minimum rate,
intermediate rates and invalid inputs. Together with the existing delegate
lifetime test, both tests pass with `native-lifetime-tests`; neither test acquires
a camera. On 2026-11-02 the user confirmed that the physical camera works on
macOS after this fix. This is manual user-reported camera-operation evidence;
it does not establish the separate photo, denial, hotplug, color/geometry or
long-running recovery gates.

## Upstream integration — 2026-11-02

The desktop branch integrates `origin/main` at
`f6ccf8b21f4abfc653b4342cfb734f796fd5e22e`, including the dispatcher/executor
updates and Graphite pipeline prewarming. The two Skia import/export conflicts
retain the upstream prewarm context/API together with media frame-context IDs,
frame leases and asynchronous GPU completion. Prewarming still uses its own
recorder; the media completion path stays on the actual rendering queue.

A fresh default Metal replay passes all **37 stages**, including both expected
negative controls and the full 300-cycle / 600-reset memory workload. All 16
orientation/crop cases, achieved seek/recovery pixels, native transport
accessibility/keyboard actions, shared media contracts, the real Metal frame
fixture, both native camera contracts and the Vision fixture pass. Default
playback/recovery/EOF restart, development bundle signing/launch, discovery and
deployment metadata pass. Five separate effect/prewarming unit tests pass too.
The integrated desktop catalog build and Metal media navigation/retirement smoke
pass. The Emscripten catalog build and real Chrome/ANGLE-Metal catalog smoke also
pass, including lazy served-video loading, native playback, fake-camera capture,
an independently decoded photo, cleanup and both QR payloads. The browser fixture
does not acquire a physical camera.

| Measurement | Cycle 10 | Cycle 300 | Change |
| --- | ---: | ---: | ---: |
| Process footprint | 196,691,144 | 214,582,472 | +17,891,328 bytes |
| Metal allocated bytes | 45,088,768 | 45,088,768 | 0 |
| Mach ports | 546 | 550 | +4 |

These changes remain below the original frozen limits. Every retirement
checkpoint reaches zero frames, GPU reads, quarantine and native player owners;
preview CPU transfers remain zero. This is the same lifecycle workload, not
full-resolution performance acceptance or universal leak-freedom evidence.

Evidence is outside Git in
`/Volumes/SSD/build-cache/ferroui-portable-media-evidence/main-integration-f6ccf8b2-2026-11-02/`.
The audit executable's SHA-256 is
`948298d45178a67c04567e88a6a3a74a99a844d49938e98887e596e97814a428`.
The JSON report links a supplemental `integration-source.json` containing both
merge parents and all staged/unstaged source hashes, frozen during the audit
build and checked unchanged after the native replay. The runner now captures
staged integration files directly on subsequent runs and requires both named
camera contracts instead of the obsolete single-test count. Later verification
documentation edits are separate from the compiled source hashes.

## Upstream integration — 2026-11-03

The desktop branch integrates `origin/main` at
`1549651d8f55f9b4cf80fcf23165dce7a0b6b96d` without text conflicts. The native
media sample (with macOS verification features) and desktop ControlCatalog
compile. The Metal catalog smoke passes playback, frame retirement on navigation,
non-acquiring camera discovery and both QR payloads.

The larger native verification run passed all 16 orientation/crop cases, its
orientation negative control and native accessibility/keyboard transport checks.
It was stopped during the memory soak at the user's request, before completing
that stage and the remaining suite. This is partial integration evidence, not a
new full-profile pass. Logs are outside Git in
`/Volumes/SSD/build-cache/ferroui-portable-media-evidence/main-integration-1549651d-2026-11-03/`.
