# Portable media mobile sample

Independent iOS and Android integration of `FerroUI.Media`, `.Video`, `.Camera`
and their separate controls/providers. The library defaults remain paused and
camera acquisition is explicit. The sample's smoke launch enables playback.
The Camera, JPEG, Stop, Detach, opt-in QR and 3D buttons exercise camera ownership and cleanup.
JPEG output is decoded independently in memory with the Rust `image` crate.

## iOS

```sh
IPHONEOS_DEPLOYMENT_TARGET=14.0 IOS_DEPLOYMENT_TARGET=14.0 \
  CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo build -p portable-media-mobile --bin portable-media-mobile --target aarch64-apple-ios-sim
scripts/ios/bundle.sh --package portable-media-mobile --bin portable-media-mobile \
  --bundle-id org.ferroui.portablemedia --no-build \
  --camera-usage-description 'Preview camera images and capture a JPEG when requested.'
xcrun simctl install DEVICE target/ios-bundles/portable-media-mobile.app
xcrun simctl launch --console DEVICE org.ferroui.portablemedia --smoke
```

Replace `DEVICE` with a booted simulator identifier. The smoke result is also in
`Documents/portable-media-report.txt` in the app's data container. Simulator
camera discovery can return no devices; this does not verify physical capture.
Device installation requires application signing and a provisioning profile.

The iOS smoke also decodes an independent synthetic two-symbol QR fixture using
Vision on retained CoreVideo input. Simulator analysis uses Vision revision 1
with CPU compute; device builds retain the default revision. This tests native
analysis without pretending that the simulator supplies a physical camera.

## Android

Set `ANDROID_HOME` and `JAVA_HOME` for the SDK/JDK described in
`docs/porting/android-platform.md`. Use Gradle 8.13 with the sample's pinned
Android Gradle plugin 8.13.0. Native compilation precedes APK packaging:

```sh
. scripts/android/env.sh
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo build -p portable-media-mobile --lib --target aarch64-linux-android
gradle --no-daemon -p samples/PortableMedia.Mobile/android \
  -PferroUiRoot="$PWD" assembleDebug
adb -s DEVICE install -r target/android-media-gradle/outputs/apk/debug/PortableMediaMobile-debug.apk
adb -s DEVICE shell am start \
  -n org.ferroui.portablemedia/org.ferroui.android.FerroMainActivity --ez mediaSmoke true
adb -s DEVICE shell run-as org.ferroui.portablemedia cat files/portable-media-report.txt
```

The smoke route first creates one independent GPU texture and presents it in
two views. It waits for that allocation to retire before opening AVC/AAC media.
Native SurfaceTexture conversion uses the compositor's EGL context and GPU
fences. Media3 supplies native decoder/audio timing and per-frame presentation
timestamps on a dedicated player looper; command intent is coalesced and release
does not wait on the UI thread. No CPU preview route is declared. CameraX acquisition uses a separate
crate and Gradle dependency script, with no dependency on the video provider.

For the 16 KiB gate, verify `adb -s DEVICE shell getconf PAGE_SIZE` prints
`16384`, check all ELF load-segment alignments, and run:

```sh
zipalign -c -P 16 4 target/android-media-gradle/outputs/apk/debug/PortableMediaMobile-debug.apk
```

The Gradle scripts next to each provider show application integration without
this sample. Add only the common media script and the desired video/camera
script. The camera manifest needs CAMERA permission and an optional camera
feature declaration. No microphone or photo-library permission is requested.

The Gradle-free `scripts/android/apk.sh` also accepts repeatable
`--java-source-dir` arguments for dependency-free Java extensions. Media3 and
CameraX applications need Gradle's AAR dependency and manifest/resource merging.

## Evidence boundaries

Smoke results prove only the checks reported by that execution. A playback
smoke pass does not establish camera capture, native accessibility, color and
orientation conformance, physical audio output or profile performance.
Additional device/runtime evidence is recorded separately in the media
platform handover document.

Optional Android QR packaging applies `FerroUI.Media.Qr.Android/packaging.gradle`.
It selects the bundled `com.google.mlkit:barcode-scanning:17.3.0` artifact, with no
runtime model download. The independent QR fixture is analysis-only CPU input;
it never enters the preview route. The QR button separately enables CameraX
native analysis, whose metadata statistics and preview transfer counters are
recorded in the sample report. SDK-internal analysis copying remains opaque.
The test fixtures contain only synthetic framework-specific text.

The 3D action reuses the existing player/camera mailbox in a client-owned
transparent projected plane. The framework's renderer host only schedules native
encoding and presents its shared output; the scene and shaders are sample code.
`--renderer-smoke` (iOS) or `--ez rendererSmoke true` (Android), alongside the
normal smoke option, enables counted diagnostic projection/alpha probes.
These CPU probes run only for explicit smoke testing and are excluded from
preview transfer counts and performance acceptance.

The tested emulator camera provides an unknown dataspace. For that synthetic
source, launch Android with `--ez cameraSdrSrgb true` to explicitly declare its
sRGB interpretation. This does not claim measured physical-camera color accuracy.
Native color tags always take priority; ordinary applications leave the provider
option None unless they have an independently known source contract. Untagged
sources are rejected by default, and tagged HDR/wide-gamut sources remain outside
this SDR profile. Native dataspace discovery requires Android 33; an explicitly
known source profile can be supplied on earlier framework-supported versions.

## Paused saturation regression

Add `--saturation-smoke` to the iOS `--smoke` launch, or add
`--ez saturationSmoke true` to the Android `mediaSmoke` launch. Keep renderer
smoke disabled for this case. The sample retains three distinct source leases,
pauses and seeks to 750 ms with the frame budget full. It checks pending work
past 1.5 seconds, releases the leases after two seconds and requires a new paused
frame before resuming. Stop must still present frame zero and teardown must
retire all source storage. This tests real provider saturation, separately from
mocked completion or a diagnostic GPU readback.

## Video geometry regression

Add `--geometry-smoke` with `--smoke --renderer-smoke` on iOS to select the
synthetic 90° rotated and mirrored AVC/AAC fixture. Add `--crop-smoke` to use its
off-centre clean-aperture variant. The Apple checks require the native transform,
visible crop and natural size to match independently specified fixture geometry.
The renderer diagnostics additionally check four projected quadrant colours,
using fixed expected positions and colours rather than the framework mapping.
Android accepts the corresponding `geometrySmoke` and `cropSmoke` launch extras.
Its 16 KiB emulator run passed the reflected/cropped fixture using the native
Media3 pipeline and independent projected quadrant probes. Native output may
already contain rotation; the remaining reflection/crop are reported in metadata.

Generate new copies without replacing existing fixtures:

```sh
swift tests/media/make-fixture.swift /path/to/rotated.mp4 --rotate90 --mirror
swift tests/media/make-fixture.swift /path/to/cropped.mp4 --rotate90 --mirror --crop
```

These test-only producer operations allocate synthetic CPU pixels. Production
preview retains its zero CPU-transfer policy. Metal diagnostic mapping counts
the mapped output allocation; GLES diagnostic probes count 13 RGBA pixels
(52 bytes). No diagnostic probe runs during ordinary playback/camera presentation.

Host metadata/orientation contracts (no Android device or decoder):

```sh
javac -d /tmp/media-geometry-tests \
  src/FerroUI.Media.Video.Android/java/org/ferroui/media/video/Mp4Geometry.java \
  src/FerroUI.Media.Android/java/org/ferroui/media/FrameOrientation.java \
  tests/media/android/Mp4GeometryContract.java tests/media/android/FrameOrientationContract.java
java -cp /tmp/media-geometry-tests Mp4GeometryContract tests/media/fixtures
java -cp /tmp/media-geometry-tests FrameOrientationContract
```

The 3D toggle records requested state separately from temporary source
availability. Stop retires its output; restarting/replacing the camera restores
the requested renderer when the replacement input becomes available. Native
smoke completion requires the renderer probe when enabled, and any earlier
reported smoke failure prevents a final success result.
