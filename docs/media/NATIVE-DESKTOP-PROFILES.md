# Native desktop media profiles

The Windows and Linux provider source, shared native-alias route and desktop
sample routing are present. Native C/C++ compilation, linking, packaging and
runtime evidence are tracked separately in the platform sections below. Rust cross-checks
validate the provider bindings; they do not establish that the SDK bridges
compile or that codecs/drivers negotiate an admitted GPU route. The initial Windows
sample cross-check stopped at a missing MinGW C compiler on the Mac; receiving-host
evidence supersedes that initial build status for the profiles actually exercised.

The implementation plan remains in chat. This document records implemented
interfaces, build dependencies and current limitations.

## Packages and renderer admission

Video and Camera are independent additions to Media. Their platform packages
use `FerroUI.Media.Windows` or `FerroUI.Media.Linux` for retained storage and
interop. Linux Camera owns the portal/PipeWire development dependencies; Linux
Video does not depend on them. The common controls/importer are optional through
`controls`. No provider declares a CPU preview fallback; GPU-preferred and
GPU-only both reject a missing admitted GPU route in these profiles.

| Profile | Implemented source route | Still open |
| --- | --- | --- |
| Windows ANGLE/EGL + GLES3 | Active D3D11 device → MF DXGI manager/Source Reader → retained ARGB32/BGRA DXGI sample → EGL texture alias → leased direct sampling | Physical camera, wider codec/driver profiles and remaining acceptance in WINDOWS-VERIFICATION.md |
| Linux EGL + GLES3 | Exact DMA_DRM caps from the active display → retained GStreamer DMA-BUF → direct XRGB/XBGR alias, or counted NV12 GPU conversion → leased sampling | Native build passed; WSL lacks DMA-BUF import; positive GPU/runtime gates remain open |
| Linux EGL + desktop GL | Explicitly negotiated XRGB/XBGR direct aliases | NV12 conversion shader profile and native verification |
| Linux default X11 GLX, Vulkan; Windows WGL/Vulkan | No direct media route added | Completion/import implementation and profile acceptance |

The Linux sample explicitly requests EGL/GLES on X11. This does not change the
framework's default rendering mode or establish support for its GLX/Vulkan
profiles. Wayland EGL uses the same importer contract; it has no native evidence
from this task.

`NativeGlFrameProducer::import_native` reserves storage credits before creating
an alias. Final lease/GPU completion queues alias/backing destruction on the
owning context. Credits survive that queued cleanup. Context-loss cleanup uses
native-only release and quarantines accounting. A dropped uncommitted view rolls
back on its owning context. This route allocates no framework image copy.

## Windows

Receiving-host build, playback/renderer/recovery, independent native pixels,
packaging and remaining acceptance gates are recorded in
[WINDOWS-VERIFICATION.md](WINDOWS-VERIFICATION.md).

Build on a Windows machine with the MSVC Rust target, Visual C++ tools and a
current Windows SDK. Use the repository's normal ANGLE/runtime packaging. No
private decoder device is created. The compositor first attempts a D3D11 device
with BGRA and video support, retaining its ordinary graphics fallback for UI
profiles that cannot provide it; media independently rejects that fallback.

```powershell
cargo build -p portable-media --target x86_64-pc-windows-msvc
cargo run -p portable-media -- tests/media/fixtures/avc-aac.mp4 --resource --smoke
cargo run -p portable-media -- tests/media/fixtures/avc-aac.mp4 --renderer
cargo run -p portable-media -- --camera-discovery
cargo run -p portable-media -- --camera
```

The media bridge probes D3D11 video/threading support, enables immediate-context
multithread protection and rejects software adapters. `MFCreateDXGIDeviceManager`
is resolved at runtime so an unavailable newer media entry point does not become
a mandatory application import on older framework UI profiles. Other missing
capabilities are reported by the provider instead of raising framework floors.

One MF source reader supplies both audio and video. Native advanced ARGB32/BGRA video
processing is requested on that DXGI manager. Each video sample must expose an
`IMFDXGIBuffer` from the exact device with compatible, shader-readable BGRA
storage, known SDR color tags, dimensions, aperture and array slice. A native
D3D event query establishes producer readiness; consuming GL fences establish
GPU retirement. Audio is PCM through WASAPI, with its native audio clock driving
playback timing. Decode acceleration remains Unknown; a DXGI output alone is not
proof of hardware decoding or of opaque codec/driver transfer behavior.

The GPU conversion counter records the requested native RGB-processing boundary
per admitted sample. Inspect the native MFT/driver path before making stronger
zero-copy or performance claims about its internals. Unknown color tags,
incompatible surfaces and unsupported aperture/pixel-aspect conversions fail.

Camera discovery enumerates MF activation metadata without activating capture.
Start activates only the selected video device; microphone access is not
requested. Consent/privacy failures come from native acquisition. Formats report
the actual capture-output type. Torch, zoom, focus and exposure are unavailable
in this profile. JPEG stills are explicitly `PreviewSnapshot`, not native stills.
Only explicit photo work creates a staging texture/maps pixels. It runs on a
worker and returns independently owned encoded bytes. Unknown GPU completion
quarantines both source and staging and occupies the native photo slot, bounding
repeated failures.

An opt-in `synthetic-camera` feature now exercises that same native capture path
with an in-process MF source generating immutable D3D11 GPU quadrant frames.
JPEG pixels, saturation/resume, cancellation, restart, renderer sharing and an
injected MF disconnection are covered in the receiving-host verification note.
The fixture is absent from default builds and is not an OS camera registration
or evidence of privacy/hardware behaviour.

The optional `--camera-recovery-smoke` extension exercises owned/borrowed
`CameraPreview` controls, native MF loss, failed factory retry and cancellation
over six compositor resets. Independent GL, composition and window-only pixel
checks cover changing palettes, perspective/premultiplied alpha, resize and all
three stretch modes; explicit diagnostic reads are counted separately from
zero-CPU preview. See [WINDOWS-VERIFICATION.md](WINDOWS-VERIFICATION.md) for
commands, evidence and the distinction from physical driver/display acceptance.

Source-file/resource preparation is limited to four filesystem workers;
cancellation retains those slots through the actual write.
Native worker limits are four sessions, three future video samples, one native
latest frame, one source-reader callback, bounded PCM queues (384,000 bytes hard
limit) and a WASAPI buffer of at most one second. The public frame budget also
bounds admitted leases and old generations. A decoder texture array is charged
conservatively in full; driver padding/pools remain opaque. Native source opening
can remain inside an OS call after cancellation; the bounded worker slot stays
occupied until that call returns. Verify cancellation/retirement latency on the
receiving host.

## Linux

Native WSL build, descriptor/ownership tests and capability rejection results
are recorded in [LINUX-VERIFICATION.md](LINUX-VERIFICATION.md). This WSL display
lacks DMA-BUF import; positive native GPU/camera gates remain open.

The media capability profile requires system GStreamer 1.24+ development/runtime
packages, its app/video/allocators libraries, EGL/GLES and appropriate native
codec/output plugins. This is a narrower optional media capability, not a change
to the framework's OS floor. Camera additionally needs GIO Unix FD support,
PipeWire development/runtime packages, a camera portal backend and GStreamer's
PipeWire/V4L2 plugins. There is no bundled GStreamer or silent plugin download.

```sh
cargo build -p portable-media
cargo run -p portable-media -- tests/media/fixtures/avc-aac.mp4 --resource --smoke
cargo run -p portable-media -- tests/media/fixtures/avc-aac.mp4 --renderer
cargo run -p portable-media -- --camera-discovery
cargo run -p portable-media -- --camera
cargo run -p portable-media -- --camera --v4l2=/dev/video0
```

Playback uses one `playbin3` pipeline with native audio/timing and an appsink
requesting exact active-EGL format/modifier caps. Source preparation is also bounded to four filesystem workers.
No CPU `videoconvert`, video
map, staging or upload enters preview. Appsink allocation queries request video
metadata. Each retained sample validates FD allocation extents, offsets, strides,
known modifier/format, progressive scan, square pixels, crop and SDR color.
Compatible XRGB/XBGR creates a direct alias. NV12 uses EGL's native BT.709
range/matrix interpretation and a counted external-texture GPU conversion to
immutable RGBA storage; Rec.709 sampling normalization remains on the GPU.
Unknown color, implicit layout and unsupported conversions are rejected.

Seeks advance epochs immediately, clear queued output and use downstream segment
sequence numbers to reject old samples. Actual native state/position/duration and
seekability are queried on the worker. Appsink and native latest slots are each
one frame; four native workers and one queued import per stream bound pending
work. PipeWire requests a buffer pool with a maximum of six buffers. Native pool
allocation and driver overhead still need measurement alongside frame-budget
counters.

Portal discovery reads IsCameraPresent without access or hardware acquisition.
Explicit start requests AccessCamera, closes canceled request handles and opens
only authorized remote connections. An authorized remote is consumed once for
node enumeration, then a fresh remote is opened for capture. The current public
portal selection is `portal:default`; post-consent multi-device selection is not
exposed yet. A direct V4L2 profile uses only explicitly configured paths, with
DMA-BUF I/O. It never becomes a fallback for portal denial. Camera controls are
reported unavailable. JPEG output is an independently encoded preview snapshot;
only linear SDR buffers up to 4096×4096 admit that photo path. CPU mapping,
RGB repacking and encoded output are counted separately as Photo transfers.

The native bridges have no release-gate evidence for this profile. Validate
missing plugins, denial/cancellation, stale callbacks, hotplug, modifiers,
software rejection, negative transfer controls, actual completion, two consumers,
resize/device loss, photo decoding, native audio/accessibility and frozen
performance workloads on the receiving machine.

## Mac-only binding checks

For implementation review without native SDKs:

```sh
FERROUI_NATIVE_MEDIA_CHECK_ONLY=1 cargo check \
  -p ferroui-media-windows -p ferroui-media-video-windows \
  -p ferroui-media-camera-windows --target x86_64-pc-windows-gnu
FERROUI_NATIVE_MEDIA_CHECK_ONLY=1 cargo check \
  -p ferroui-media-linux -p ferroui-media-video-linux \
  -p ferroui-media-camera-linux --target x86_64-unknown-linux-gnu
```

This explicit switch skips native compilation and emits a warning. It is only
for `cargo check`; it cannot produce a working native provider binary. Unset it
for every native build/test/release gate. Native verification and platform
release acceptance remain separate from these binding checks.
