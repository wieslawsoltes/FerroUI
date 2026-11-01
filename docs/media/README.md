# Portable media implementation

The normative behavior is in [the specification](PORTABLE-MEDIA-FRAMEWORK-SPECIFICATION.md).
This is an initial implementation, not a declaration of six-platform support.
The [platform-agent handover](PLATFORM-HANDOVER.md) records the receiving branches,
integration boundaries, reproducible checks and outstanding native work.

## Available code

- `ferroui_base::rendering::composition::frames` provides immutable frame metadata,
  shared native ownership, bounded budgets, latest-frame publication, asynchronous
  GPU completion and presentation admission/transfer accounting. The default
  budget permits three allocations, 128 MiB and six submitted GPU reads, including
  repeated reads of one frame. These are configurable safety limits, not verified
  performance thresholds. Quarantined reads remain charged and inspectable.
- `CompositionDrawingSurface::update_shared_frame_async` publishes an immutable
  imported frame without taking a snapshot. Unsupported backends return an error.
  Metal/Skia Graphite wraps the existing texture and retains native storage until
  an asynchronous completion on the actual Skia command queue.
  Page-owned Skia WebGL2 borrows textures in the active context and retires reads
  through native fences polled with zero timeout. Reservations bound allocation
  before GPU storage creation; retirement tokens cover queued native cleanup.
  `LeasedPlatformHandle` keeps native source storage alive while composition
  carries its handle across a queued import, including cancellation.

The packages are independently installable and use the `FerroUI.Media.*` family:

| Crate directory | Cargo package | Responsibility |
| --- | --- | --- |
| `FerroUI.Media` | `ferroui-media` | Shared frames, events, errors and renderer host; no playback/capture provider |
| `FerroUI.Media.Video` | `ferroui-media-video` | MediaSource, MediaPlayer and MediaPlaybackSession; depends on Media |
| `FerroUI.Media.Camera` | `ferroui-media-camera` | Discovery, explicit capture, negotiated settings and owned photos; depends on Media |
| `FerroUI.Media.Qr` | `ferroui-media-qr` | Optional QR analysis; depends on Media, independently of video/camera providers |
| `FerroUI.Media.Controls` | `ferroui-media-controls` | Common TexturePresenter and renderer-host control |
| `FerroUI.Media.Video.Controls` | `ferroui-media-video-controls` | MediaPlayerElement and MediaTransportControls |
| `FerroUI.Media.Apple` | `ferroui-media-apple` | Shared CoreVideo ownership and IOSurface imports |
| `FerroUI.Media.Video.Apple` | `ferroui-media-video-apple` | AVFoundation playback, native audio and timing |
| `FerroUI.Media.Camera.Apple` | `ferroui-media-camera-apple` | AVFoundation camera acquisition and JPEG capture |
| `FerroUI.Media.Browser` | `ferroui-media-browser` | Shared WebGL2 ownership, producer fences and direct texture imports |
| `FerroUI.Media.Video.Browser` | `ferroui-media-video-browser` | Native browser playback/audio and shared frame output |
| `FerroUI.Media.Metal` | `ferroui-media-metal` | Client-owned native Metal encoding, bounded IOSurface output and completion |
| `FerroUI.Media.OpenGL` | `ferroui-media-opengl` | Active-context GPU producer jobs and deferred texture retirement |
| `FerroUI.Media.Android` | `ferroui-media-android` | Shared external GLES conversion and surface ownership |
| `FerroUI.Media.Video.Android` | `ferroui-media-video-android` | Independent Media3 playback and native presentation timestamps |
| `FerroUI.Media.Camera.Android` | `ferroui-media-camera-android` | Independent CameraX acquisition and native JPEG |
| `FerroUI.Media.Camera.Controls` | `ferroui-media-camera-controls` | Explicit-start CameraPreview and weak supplied sessions |
| `FerroUI.Media.Qr.Apple` | `ferroui-media-qr-apple` | Optional native Vision analysis of retained CoreVideo storage |
| `FerroUI.Media.Qr.Android` | `ferroui-media-qr-android` | Optional bundled offline ML Kit analysis of borrowed CameraX images |
| `FerroUI.Media.Windows` | `ferroui-media-windows` | Retained MF/DXGI storage and compatible ANGLE device import; native build pending |
| `FerroUI.Media.Video.Windows` | `ferroui-media-video-windows` | MF source reader, WASAPI audio/clock, shared video leases |
| `FerroUI.Media.Camera.Windows` | `ferroui-media-camera-windows` | Metadata discovery, explicit MF capture and JPEG preview snapshots |
| `FerroUI.Media.Linux` | `ferroui-media-linux` | System GStreamer, negotiated DMA-BUF storage and EGL import; native build pending |
| `FerroUI.Media.Video.Linux` | `ferroui-media-video-linux` | One native GStreamer video/audio pipeline |
| `FerroUI.Media.Camera.Linux` | `ferroui-media-camera-linux` | Separate portal/PipeWire dependencies, explicit V4L2 profiles, JPEG snapshots |
| `FerroUI.Media.Camera.Browser` | `ferroui-media-camera-browser` | Explicit video-only acquisition, preview and owned JPEG capture |

Video and camera do not depend on each other. Installing Video brings the shared
Media base; installing Camera adds capture contracts separately. UI controls and
native providers are separate dependencies. The native Apple providers expose an
optional `controls` feature for the common AppleTextureImporter. Camera's native
acquisition code is absent from the video provider, and playback code is absent
from the camera provider.
The browser providers likewise contain separate playback and capture modules;
neither imports the other. `BrowserCameraProvider::discovery()` works before a
compositor exists and never acquires hardware. Starting capture requires a
producer admitted from the active composition interop.

Playback handles reentrant events, preparation-time intent, latest-seek/source
cancellation and native achieved seek positions. Views release only their own
subscriptions and frame leases for caller-owned sessions. A Rust-supplied custom
placeholder/poster is normal styled content. Current video markup properties are
Stretch, AutoPlay and AreTransportControlsEnabled; providers/sessions are supplied
from Rust.

Apple camera configuration/start/stop run on a serial capture queue. It reports
fractional negotiated frame rates, permission outcomes, disconnect/runtime errors
and native JPEG capability; optional device controls are reported unavailable.
The compositor device identity must be supplied from the active renderer. Decode
acceleration remains unknown. Unknown native color metadata is rejected.
Frame timestamps come from AVFoundation's decoded-item display time, and
imports reject a retired renderer generation even when its adapter identity matches.

`RendererHost` uses normal composition, routed input and styling. It schedules
client-owned rendering on invalidation or continuously, maps logical pointer
coordinates to the active pixel viewport, and retires output on hide/detach.
Clean static hosts stop their timer; disposable source notifications wake the
owning UI dispatcher. Active work coalesces to the render timer, and ancestor
visibility/scaling changes invalidate or suspend it. Rebinding inputs and
resizing advance publication generations. Input mailboxes
borrow existing frames; they never open another player or camera. Metal and
native GL adapters allocate bounded premultiplied output on the active device,
retain all inputs until actual completion and reject incompatible devices.
Client scenes, depth/MSAA storage, native libraries and hit testing stay outside
the framework. `project_frame_point` applies crop/orientation and a client planar
matrix to source/QR corners. Non-planar mapping stays client-supplied.

Native camera texture dimensions on Android are matched to Camera2 sensor
timestamps, including CameraX use-case rebinds. A pure orientation decomposition
combines the desired transform with the transform already sampled from
SurfaceTexture, so rotation/mirroring is applied once. Tagged SDR BT.709 and
BT.601 profiles are normalized to sRGB in the required GPU conversion pass.
Unknown, wide-gamut and HDR tags are rejected. Dataspace tags require API 33;
for an independently known untagged source, `AndroidCameraProvider` accepts an
explicit `untagged_sdr_color` contract (sRGB or Rec.709). Native tags take priority.
The default is None, and does not guess a color space. The mobile sample's
`cameraSdrSrgb` launch option declares the synthetic emulator source explicitly;
it is not physical-camera color evidence. Framework OS floors are unchanged.

The optional Rust rxing adapter bounds analysis, rejects stale results and handles
multiple symbols. Packed luma is borrowed; adapter stride/pixel repacking is counted
separately from preview. Corrected codewords and byte-mode segments are distinct
from full payload bytes, which this adapter does not currently expose. Finder
extrapolation is labeled estimated geometry. Expiry uses acquisition age and the
caller's scheduler clock, independently of source media timestamps.
Internal decoder input-copy behavior still needs profiling beyond adapter repacking.

Native Apple QR uses one bounded Vision worker; cancellation retains its input
until native recognition finishes and rejects late publication. Analysis geometry
stays in the unrotated source plane. Device builds use Vision's default revision;
the tested iOS simulator requires revision 1 with CPU compute. This is analysis
compute, independent of the GPU preview policy. Android QR adds CameraX
ImageAnalysis only when explicitly attached. It borrows native YUV images,
closes every ImageProxy after actual task completion, keeps only the latest
queued image and globally limits recognition to one task at 10 Hz. Results expire
after 250 ms. Sensor/buffer and SurfaceTexture transforms map corners to the
preview texture without applying orientation twice. Non-full analysis crops are
rejected. Native SDK internal copies are opaque; zero framework transfer counters
do not establish zero internal copying. The optional ML Kit artifact is bundled
in the APK; no model installation/download metadata is added.

The [mobile integration sample](../../samples/PortableMedia.Mobile/README.md)
contains explicit camera, JPEG, Stop, Detach and QR actions. Its native QR smoke
uses a synthetic analysis-only fixture; this is separate from camera acquisition
and GPU preview. The framework defaults keep QR disabled.

## Native sample and checks

On macOS, run `cargo run -p portable-media -- /absolute/path/to/video.mp4`.
The independent sample explicitly enables automatic playback and transport controls.
Add `--renderer` to reuse the same source in a transparent, projected Metal
plane. With `--smoke`, counted diagnostic pixel probes check independently
selected coverage/alpha points, then both source and renderer allocations must
retire. The scene/pipeline belongs to the sample.
Add `--smoke` to exercise playback, pause, achieved native seek, resume, Stop/frame-zero import, clear and
final native-frame retirement. The input remains external; private application
assets are not included in the framework. Add `--resource` with a local MP4 to
exercise owned encoded-resource playback through a scoped private temporary file.

`cargo run -p portable-media -- --camera` opens explicit Start, Stop and Capture
buttons. Captured JPEG bytes are independently decoded in memory; the sample does
not write camera images to disk. `--camera-discovery` enumerates without starting
capture and verifies that permission did not change. The macOS sample embeds its
camera usage description. Applications must supply their own usage descriptions
and platform packaging/entitlements. The native APIs require macOS 10.15 or iOS 11;
FerroUI's existing iOS 14 floor still applies. The providers compile with that
deployment target; minimum-OS linking/runtime acceptance remains pending.

Targeted checks:

```sh
cargo test -p ferroui-base --lib rendering::composition::frames
cargo test -p ferroui-base --lib composition_drawing_surface_tests
cargo test -p ferroui-media -p ferroui-media-video -p ferroui-media-camera --lib
cargo test -p ferroui-media-controls -p ferroui-media-video-controls -p ferroui-media-qr --lib
cargo test -p ferroui-skia --lib --features native-frame-tests native_shared_metal_frame
cargo test -p ferroui-media-camera-apple --lib --features native-lifetime-tests
cargo check -p portable-media
```

`native-frame-tests` enables an independent development-only Metal producer. It
writes IOSurface storage with a render pass, holds actual GPU completion pending,
samples opaque/transparent textures and validates diagnostic readback pixels.
Those readbacks are outside production preview and are counted separately.
The camera lifetime fixture exercises asynchronous native delegate/session
retirement using Foundation-only fakes; it does not acquire camera hardware.

## Browser sample and checks

Activate the pinned tools from `scripts/browser/setup.sh`, then run:

```sh
scripts/build-browser.sh portable-media-browser --debug
FERROUI_BROWSER_ANGLE=metal node scripts/browser/tests/portable_media.test.mjs
```

The site starts with an independent four-quadrant GPU producer feeding two
ordinary controls. It includes file playback, explicit camera acquisition, JPEG
capture and closing/navigation cleanup. Playback defaults remain paused; the
transport controls supply Play. Browser file input supplies owned encoded
resources; `MediaSource::File` names the Emscripten virtual filesystem. Relative
and absolute HTTP(S) URLs and caller-owned blob URLs use native browser loading,
including CORS and native seek-range restrictions.

The browser test uses real Skia/WebGL2 in Chrome with the selected ANGLE backend.
It checks deterministic pixels, sibling ordering, delayed completion notification,
retained storage after detach, native playback/seek/stop, encoded resources, native
autoplay denial/recovery, diagnostic decoded audio samples, CORS rejection,
non-prompting discovery, permission denial, synthetic camera preview, independently
decoded JPEGs, late acquisition cancellation and software-adapter rejection.
An explicit CPU-backed upload negative control must be rejected and counted
without allocating frame storage or transferring preview bytes.
Synthetic camera evidence is separate from physical-device acceptance. Delaying
fence notification is not evidence of delayed hardware execution.

`tests/media/fixtures/avc-aac.mp4` is a synthetic three-second H.264/AAC fixture
with four colored quadrants and a 440 Hz tone. The adjacent Swift generator
reproduces its content using system codecs; encoded bytes can vary by OS. No
application assets are included in this fixture.

## Capability and evidence boundaries

The direct Metal/Graphite and page-owned WebGL2 importers are implemented. Other importers retain
their existing snapshot behavior and reject the new direct acquisition API.
The current native video provider does not implement CPU presentation fallback.
Rec.709 full-range opaque RGB is converted to the compositor's untagged sRGB
values in a GPU sampling color filter using the BT.1886 reference EOTF. Unknown
color, HDR, multiplane YUV, straight alpha and bottom-origin direct imports are
not admitted by the Metal bridge. WebGL2 supports top/bottom origins, opaque and
premultiplied RGBA8 in the common importer; native video/camera output is opaque
and browser-converted to sRGB. Framework counters do not expose decoder/driver
internals or independently establish hardware acceleration. WebGL byte accounting
covers RGBA8 storage payload, while driver padding/cache overhead remains opaque;
allocation count and dimensions are bounded. Native image-to-GL transfers have
an explicit reason and counter, distinct from CPU preview transfers.

The browser profile requires a single-threaded, page-owned WebGL2 Skia context.
Workers, WebGL1, Vello and WebGPU do not advertise this route. Software adapters
are rejected; there is no declared CPU preview fallback in this profile. Native
frame output requires requestVideoFrameCallback for decoded-frame timestamps.
Pixel-aspect conversions whose decoded and upload dimensions differ are rejected;
CPU-backed image/canvas upload routes are forbidden and counted as attempted
preview transfers. Camera still output is encoded to JPEG separately
from preview, with native-still/preview-snapshot provenance, a single photo in
flight, a 4096×4096 encoding limit and a 64 MiB encoded-output limit. Native codec
work is opaque; the encoded transfer to Rust is counted as Photo. Browser camera
controls are reported unavailable. Context loss quarantines unfinished reads;
automatic recovery and a broader browser/device matrix remain unverified.

Windows and Linux provider source and desktop routing are now present. Their
Rust bindings cross-check on the Mac; native C/C++ compilation, packaging and
runtime behavior remain unverified. See [NATIVE-DESKTOP-PROFILES.md](NATIVE-DESKTOP-PROFILES.md).
Android Media3 playback, CameraX capture, bundled offline ML Kit QR and GLES
renderer output have run together on a 16 KiB emulator. Apple playback, Vision
QR and Metal renderer output have run on macOS and the iOS simulator. Apple
camera acquisition/photos compile but lack physical-device/denial/hotplug
acceptance evidence. The capture callback keeps a single
native latest-frame slot in addition to the admitted frame budget; native pool
and driver allocations have not been profiled. One native still request is
allowed across generations, with a 64 MiB encoded-output limit. Canceled native
still work retains its slot until AVFoundation's final callback.

CameraPreview, RendererHost and client-owned Metal/GLES plane examples are
implemented. Automatic device-loss recovery, complete markup/source APIs,
six-platform packaging and native accessibility acceptance remain unfinished.
The independent mobile sample combines camera, opt-in QR, JPEG capture, renderer
output and detach/stop cleanup; the broader navigation matrix remains open. Apple source-image
orientation/crop parity and audible output have not passed native acceptance.
A public contract or simulated lifetime test is not
functional support for a missing provider. Earlier media-branch evidence is not
acceptance of this implementation.
