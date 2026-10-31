# Portable media mobile sample

Independent iOS and Android integration of `FerroUI.Media`, `.Video`, `.Camera`
and their separate controls/providers. The library defaults remain paused and
camera acquisition is explicit. The sample's smoke launch enables playback.
The Camera, JPEG, Stop and Detach buttons exercise camera ownership and cleanup.
JPEG output is decoded independently in memory with the Rust `image` crate.

## iOS

```sh
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
