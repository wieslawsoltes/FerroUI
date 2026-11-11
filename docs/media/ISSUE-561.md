# Portable media controls and Gallery examples

The controls for [#561](https://github.com/wieslawsoltes/FerroUI/issues/561) use the portable providers and shared texture composition from #550. `MediaPlayerPresenter` specializes `TexturePresenter`; it adds no decoder, copy path or frame mailbox. `MediaPlayerElement.PosterSource` uses a regular image and yields to the first accepted frame. `IsFullWindow` moves the existing presentation into the current top level's overlay. Escape restores it; resize changes the overlay's bounds; detaching restores and retires the presentation. The player and its source are kept during the transition.

`MediaTransportControls` binds the named buttons and sliders of its template for playback, seeking, volume, mute and full window. The standard layout adds stop, position, volume and status as space permits. Compact layout retains the main actions, seek bar and **More playback controls**. The menu exposes 10-second backward and 30-second forward skips, fit and crop-to-fill zoom, and the playback speed, audio track, caption and cast actions supported by the active source and provider. The individual visibility properties apply to both layouts. Button, Slider and MenuItem peers expose the actions to automation. Replacing the template disconnects the old parts from transport commands.

`CameraCaptureUI` uses `CameraPreview` and starts only on an explicit action. Photo and recording completions return independently owned encoded results. Stopping, detaching or replacing the template invalidates pending capture results. AVFoundation, Windows Media Foundation, Linux GStreamer and Android CameraX can encode silent H.264 MP4 from the existing camera session, up to 60 seconds and 256 MiB. Recording requires a supported camera format and the corresponding platform encoder. The Windows and Linux recording workers consume a bounded queue of native camera samples and perform any raw conversion in the recording path. Android uses a native encoder surface and checks the finalized file with `MediaExtractor`. Preview uses the existing GPU imports, and the providers open no microphone.

The WinUI, macOS and Material themes supply the transport and capture templates. The optional control packages publish their own `WINUI_ONLY_CONTROLS` lists to avoid a dependency cycle with `ferroui-controls`. Fluent and Simple supply no templates for them. The existing independent media samples select WinUI so their transport remains available.

ControlCatalog exposes a page for each control. Its three video pages reuse the same video example and its single player. WinUI Gallery has native MediaPlayerElement, CaptureElementPreview and Sound adaptations. The video example uses the shared, attributed Sintel clip and the desktop storage picker. The Sound page plays application-owned PCM cues through the media provider. It records WinUI's Windows-specific spatial audio service as not applicable. The Gallery status describes example coverage; it does not certify every renderer or capture provider.

Run the original FerroUI Control Gallery with `cargo run --locked -p control-catalog-desktop`. The default WinUI theme includes the media templates. Open **Media & Graphics** for the MediaPlayerElement, MediaPlayerPresenter, MediaTransportControls and CameraCaptureUI pages.

Run the desktop Gallery from the workspace with:

```sh
cargo run -p winui-gallery --features desktop -- --page=MediaPlayerElement
cargo run -p winui-gallery --features desktop -- --page=CaptureElementPreview
cargo run -p winui-gallery --features desktop -- --page=Sound
```

On macOS, add `quartz-metal` to the Gallery features for the Metal media profile. The executable embeds the camera usage description. A distributed app bundle must also carry the description in its own Info.plist. The Linux Gallery tries EGL first. Its media preview requires compatible EGL/DMA-BUF support and a working camera portal.

Use the portable camera and video-control suites to verify ownership, cancellation, seek and presentation contracts. The AVFoundation encoder fixture supplies generated native samples and reads back the *encoded movie* with an independent `AVAssetReader`, then checks that cancel releases the recording slot. It does not request permission or establish physical-camera acceptance.

```sh
cargo test -p ferroui-media-camera -p ferroui-media-camera-controls -p ferroui-media-video-controls --lib
cargo test -p ferroui-media-camera-apple --lib --features native-lifetime-tests
cargo test -p ferroui-themes-winui --lib media_transport_template
cargo test -p winui-gallery --lib
```

Verification recorded with this change:

- The portable media control and camera ownership suites, the AVFoundation encoder fixture, the WinUI transport layout test and all 134 Gallery tests passed. WinUI and macOS generated theme references were regenerated and checked.
- The macOS catalog media smoke passed with both Skia and Vello GPU on Metal. Both imported real video frames with zero CPU preview transfers and retired the presentation on navigation. The camera check on that host was discovery only.
- The iOS and Android simulator playback smokes passed pause, seek, resume, stop and retirement. The Android emulator also started its synthetic camera, decoded a 1280 × 960 native JPEG, and stopped with zero retained frames, GPU reads or quarantined reads. This is simulator evidence, not physical camera acceptance.
- A 48 kHz stereo PCM fixture exercised Windows Media Foundation and WASAPI through playback, end of stream and replay. No video frames were published.

- On Windows after the review fixes, the WinUI and macOS theme suites passed their media template and document tests and their generated XAML reference checks. Across the scanner, both theme suites and WinUI Gallery, 909 library tests passed and 4 were ignored. The original desktop Control Gallery passed `cargo check` with `synthetic-camera` enabled.
- The original Windows Control Gallery passed the RedirectionSurface transport smoke with 23.7 accepted fps at normal speed on the 24 fps clip, a measured 1.49× native playback rate at the 1.5× setting, both skip actions, and a zoom and fit round trip.
- The original Android Control Gallery on an API 35 emulator completed a native silent H.264 MP4 while preview continued, retained a JPEG after Stop, cancelled a second recording, and retired its GPU frames with zero CPU preview transfers. Camera permission denial returned `Denied`. A separate fixture passed alternate audio selection, on-screen captions and Off, and native 1.5× playback.

All three WinUI Gallery pages are marked ported. Closing #561 still requires positive Linux GPU playback and camera portal runs, plus physical camera validation including iOS and Android. Native text captions and alternate audio are currently exposed by Media3 on Android. Casting uses the browser's remote playback picker for eligible URL sources. The desktop providers still need native track selection and casting, the Linux and Apple players still need playback-rate support, and a cast receiver is needed for end-to-end validation.

See the [media controls handover](MEDIA-CONTROLS-HANDOVER.md) for recording profiles, smoke commands and remaining hardware checks. The previously recorded #550 verification is evidence for the shared stack on its recorded revisions.
