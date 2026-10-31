# Portable media implementation

The normative behavior is in [the specification](PORTABLE-MEDIA-FRAMEWORK-SPECIFICATION.md).
This is an initial implementation, not a declaration of six-platform support.

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
The packages are independently installable and use the `FerroUI.Media.*` family:

| Crate directory | Cargo package | Responsibility |
| --- | --- | --- |
| `FerroUI.Media` | `ferroui-media` | Shared frames, events, errors and renderer host; no playback/capture provider |
| `FerroUI.Media.Video` | `ferroui-media-video` | MediaSource, MediaPlayer and MediaPlaybackSession; depends on Media |
| `FerroUI.Media.Camera` | `ferroui-media-camera` | Discovery, explicit capture, negotiated settings and owned photos; depends on Media |
| `FerroUI.Media.Qr` | `ferroui-media-qr` | Optional QR analysis; depends on Media, independently of video/camera providers |
| `FerroUI.Media.Controls` | `ferroui-media-controls` | Common TexturePresenter |
| `FerroUI.Media.Video.Controls` | `ferroui-media-video-controls` | MediaPlayerElement and MediaTransportControls |
| `FerroUI.Media.Apple` | `ferroui-media-apple` | Shared CoreVideo ownership and IOSurface imports |
| `FerroUI.Media.Video.Apple` | `ferroui-media-video-apple` | AVFoundation playback, native audio and timing |
| `FerroUI.Media.Camera.Apple` | `ferroui-media-camera-apple` | AVFoundation camera acquisition and JPEG capture |

Video and camera do not depend on each other. Installing Video brings the shared
Media base; installing Camera adds capture contracts separately. UI controls and
native providers are separate dependencies. The native Apple providers expose an
optional `controls` feature for the common AppleTextureImporter. Camera's native
acquisition code is absent from the video provider, and playback code is absent
from the camera provider.

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

The optional Rust rxing adapter bounds analysis, rejects stale results and handles
multiple symbols. Packed luma is borrowed; adapter stride/pixel repacking is counted
separately from preview. Corrected codewords and byte-mode segments are distinct
from full payload bytes, which this adapter does not currently expose. Finder
extrapolation is labeled estimated geometry. Expiry uses acquisition age and the
caller's scheduler clock, independently of source media timestamps.

## Native sample and checks

On macOS, run `cargo run -p portable-media -- /absolute/path/to/video.mp4`.
The independent sample explicitly enables automatic playback and transport controls.
Add `--smoke` to exercise playback, pause, achieved native seek, resume, Stop/frame-zero import, clear and
final native-frame retirement. The input remains external; private application
assets are not included in the framework. Add `--resource` with a local MP4 to
exercise owned encoded-resource playback through a scoped private temporary file.

`cargo run -p portable-media -- --camera` opens explicit Start, Stop and Capture
buttons. Captured JPEG bytes are independently decoded in memory; the sample does
not write camera images to disk. `--camera-discovery` enumerates without starting
capture and verifies that permission did not change. The macOS sample embeds its
camera usage description. Applications must supply their own usage descriptions
and platform packaging/entitlements. This provider profile requires macOS 10.15
or iOS 11; it does not change the framework's OS floors.

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

## Capability and evidence boundaries

Only the direct Metal/Graphite importer is implemented. Other importers retain
their existing snapshot behavior and reject the new direct acquisition API.
The current native video provider does not implement CPU presentation fallback.
Rec.709 full-range opaque RGB is converted to the compositor's untagged sRGB
values in a GPU sampling color filter using the BT.1886 reference EOTF. Unknown
color, HDR, multiplane YUV, straight alpha and bottom-origin direct imports are
not admitted by this initial bridge.

Windows/Linux/Android/browser providers and native QR engines are not implemented
in this new framework. Apple camera acquisition/photos compile but lack physical
device/denial/hotplug acceptance evidence. The capture callback keeps a single
native latest-frame slot in addition to the admitted frame budget; native pool
and driver allocations have not been profiled. One native still request is
allowed across generations, with a 64 MiB encoded-output limit. Canceled native
still work retains its slot until AVFoundation's final callback.

Camera preview controls, a complete native renderer-host example, automatic
device-loss recovery, the complete markup/source API, six-platform packaging and
native accessibility acceptance also remain unfinished. The independent sample
does not yet cover combined navigation and QR handling. Apple source-image
orientation/crop parity and audible output have not passed native acceptance.
A public contract or simulated lifetime test is not
functional support for a missing provider. Earlier media-branch evidence is not
acceptance of this implementation.
