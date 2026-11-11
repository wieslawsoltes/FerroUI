# Media controls handover for #561

Windows and Linux record silent H.264 MP4 from the existing native camera session. Each recorder bounds input buffering and limits a clip to 60 seconds and 256 MiB. The recording workers have their own sample conversion and encoding paths. Preview continues to admit and import native GPU frames through the existing renderer. Stopping the camera, detaching the control, losing the device or changing the negotiated capture format cancels the recording and discards pending output. The camera providers never open a microphone.

## Windows

Media Foundation recording requires a supported H.264 encoder and a progressive camera format with even dimensions up to 1920 × 1080 at up to 60 fps. Formats with native rotation metadata outside the recorder's profile still support preview and photo where admitted.

Run the catalog with ANGLE and D3D11:

```powershell
$env:FERROUI_SMOKE_RENDERING = 'angle'
$env:FERROUI_SMOKE_MEDIA = '1'
cargo run --locked -p control-catalog-desktop
```

For the in-process Media Foundation fixture:

```powershell
$env:FERROUI_SMOKE_MEDIA_CAMERA = '1'
$env:FERROUI_SMOKE_MEDIA_SYNTHETIC_CAMERA = '1'
cargo run --locked -p control-catalog-desktop --features synthetic-camera
```

Clear `FERROUI_SMOKE_MEDIA_SYNTHETIC_CAMERA` to test a physical camera. A camera-enabled run checks preview retirement and exercises photo and supported recording through `CameraCaptureUI`. Look for `CATALOG_CAMERA_UI_PHOTO_PASS`, `CATALOG_CAMERA_UI_RECORDING_PASS` and `CATALOG_MEDIA_SMOKE_PASS`.

In a fresh PowerShell session, measure playback cadence from the bundled 24 fps clip using the original FerroUI Control Gallery's default Windows composition selection:

```powershell
$env:FERROUI_SMOKE_MEDIA = '1'
$env:FERROUI_SMOKE_MEDIA_CADENCE = '1'
$env:FERROUI_SMOKE_MEDIA_PAGE = 'MediaPlayerElement'
cargo run --locked -p control-catalog-desktop
```

Repeat with `MediaPlayerPresenter` and `MediaTransportControls`. The cadence smoke samples at least three seconds after startup. It checks presentation rate, the share of decoded frames accepted by the compositor and the longest gap between accepted frames.

To run the same check with RedirectionSurface, set `FERROUI_SMOKE_RENDERING=angle` and `FERROUI_SMOKE_COMPOSITION=redirection`. The render timer carries late wakes into the next deadline. DWM-composited ANGLE window surfaces use swap interval zero, including when EGL recreates the surface after resize.

Set `FERROUI_SMOKE_MEDIA_TRANSPORT=1` with the `MediaTransportControls` page to exercise the overflow menu, measure the 1.5× rate, check both skips and confirm that zoom restores the original stretch. Windows audio playback rate requires a WASAPI endpoint that supports stream rate adjustment.

## Android

The original Control Gallery's `native-media` feature installs the Android providers. Package it with the CameraX and Media3 Gradle dependencies. From the repository root, with the Android SDK, NDK and Gradle installed:

```sh
source scripts/android/env.sh
cargo build --locked -p control-catalog-android --features native-media --target aarch64-linux-android
gradle -p samples/PortableMedia.Mobile/android -PferroUiRoot="$PWD" -PferroUiMediaCatalog assembleDebug
```

For an x86_64 emulator, use `x86_64-linux-android` and `-PferroUiAbi=x86_64`. If Cargo uses a separate target directory, pass it as `-PferroUiRustTargetDir`. Use host GPU mode for the emulator. The software GL path rejects native media preview.

After installing the APK, grant camera permission when prompted. The smoke explicitly starts `CameraCaptureUI`, takes a photo, validates a silent H.264 MP4, cancels a second recording and checks preview retirement.

```sh
adb shell am start -S -n org.ferroui.controlcatalog/org.ferroui.android.FerroMainActivity \
  --es FERROUI_SMOKE_MEDIA_CAMERA 1
adb logcat -s ferroui-catalog
```

Add `--es FERROUI_CAMERA_SDR_SRGB 1` for a known sRGB emulator whose camera buffers are untagged. Look for `CATALOG_CAMERA_UI_RECORDING_PASS` and `CATALOG_CAMERA_PASS`. For transport selection, set `FERROUI_SMOKE_MEDIA_TRACKS` to an app-readable SDR H.264 MP4 path with two audio tracks and a text track containing `FerroUI caption fixture` during the first several seconds. Look for `CATALOG_TRACKS_PASS`.

## Linux

GStreamer recording requires the OpenH264 encoder, H.264 parser and MP4 muxer plugins. The recorder accepts admitted linear DMA-BUF camera frames with even dimensions from 16 × 16 through 1920 × 1080 at up to 60 fps. The portal controls acquisition and consent.

```sh
FERROUI_CATALOG_WAYLAND=0 FERROUI_SMOKE_RENDERING=egl FERROUI_SMOKE_MEDIA=1 \
  cargo run --locked -p control-catalog-desktop
```

Repeat with `FERROUI_SMOKE_MEDIA_CAMERA=1` on an EGL display that can import the negotiated DMA-BUF format and modifier, with a working camera portal.

## Verification

| Platform and profile | Result |
| --- | --- |
| Windows ControlCatalog, Skia on ANGLE/D3D11 | Media Foundation playback imported the bundled video with zero CPU pixel transfers. Navigation retired all frames and GPU reads. Discovery found zero physical cameras without acquiring one. |
| Windows ControlCatalog, default composition | MediaPlayerElement, MediaPlayerPresenter and MediaTransportControls measured 24.2, 23.6 and 24.1 accepted frames per second respectively. Each sample accepted every frame in its measured sequence span and the longest gap was 86.5 ms. |
| Windows ControlCatalog, subsequent default and WinUIComposition runs | Cadence varied across runs. The default path and forced WinUIComposition measured 4.6–6.2 fps in the later runs and missed the cadence target. |
| Windows ControlCatalog, forced ANGLE RedirectionSurface | MediaPlayerElement, MediaPlayerPresenter and MediaTransportControls passed at 22.9, 23.3 and 21.7 accepted frames per second on the bundled 24 fps clip. The longest gap was 99.7 ms. The previous MediaPlayerElement result was 15.8 fps. |
| Windows ControlCatalog, synthetic Media Foundation camera on ANGLE/D3D11 | Preview used GPU imports with zero CPU preview transfers and retired on navigation. `CameraCaptureUI` returned a photo that stayed visible after Stop and produced a silent MP4. Both catalog runs reported `CATALOG_MEDIA_SMOKE_PASS`. |
| Windows ControlCatalog, compact MediaTransportControls | The native Windows UI Automation walk found the Play, Mute, Enter full window and Playback position names. All four were exposed as keyboard-focusable. |
| Windows Media Foundation, D3D11 fixture | An independent Source Reader decoded the generated 320 × 240 H.264 MP4 and found no audio track. |
| Windows Media Foundation, synthetic camera | The native camera session continued delivering preview frames during recording. The MP4 remained usable after Stop, which cancelled a second recording. |
| Windows Media Foundation and WASAPI, stereo PCM | Playback, end-of-stream and replay succeeded with zero video frames. |
| Linux GStreamer, generated linear buffer fixture | The native session encoded 320 × 240 H.264. An independent GStreamer pipeline demuxed and decoded the silent MP4. The copied output stayed usable after Stop. |
| Linux WSL2, EGL with llvmpipe | The catalog rejected the profile with `Software/unknown GL has no admitted GPU media route`. |
| Linux WSL2, EGL with D3D12 | The catalog rejected the profile with `No explicitly negotiated native DMA-BUF formats`. |
| Physical cameras and portal denial | Windows reported no camera. The Linux host exposed no camera device or desktop portal. |

Run the native encoder fixtures through Cargo:

```sh
cargo test --locked -p ferroui-media-windows --features native-frame-tests --lib
cargo test --locked -p ferroui-media-linux --features native-frame-tests --lib
```

The Linux fixture prints a skip message if the required GStreamer encoder and muxer plugins are absent. The Windows fixture requires a hardware D3D11 device.

## Remaining hardware acceptance

- Run the Linux playback and camera checks on a host with compatible EGL/DMA-BUF support and a working portal. Verify that navigation retires GPU frames and that preview records zero CPU pixel transfers.
- On a host with a physical camera, verify explicit start, still capture, silent recording, permission denial and navigation away while acquisition is pending.
