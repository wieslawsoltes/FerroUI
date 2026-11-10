# Portable media controls and Gallery examples

The controls for [#561](https://github.com/wieslawsoltes/FerroUI/issues/561) use the portable providers and shared texture composition from #550. `MediaPlayerPresenter` specializes `TexturePresenter`; it adds no decoder, copy path or frame mailbox. `MediaPlayerElement.PosterSource` uses a regular image and yields to the first accepted frame. `IsFullWindow` moves the existing presentation into the current top level's overlay. Escape restores it; resize changes the overlay's bounds; detaching restores and retires the presentation. The player and its source are kept during the transition.

`MediaTransportControls` binds the named buttons and sliders of the WinUI template. The standard layout adds position text, status and volume as space permits. Compact layout retains the main actions and seek bar. The individual action and slider visibility properties apply to both. The existing Button and Slider peers expose the actions, with current Play/Pause, Mute/Unmute and Enter/Exit full window names. Replacing the template disconnects the old parts from transport commands.

`CameraCaptureUI` uses `CameraPreview` and starts only on an explicit action. Photo and recording completions return independently owned encoded results. Stopping, detaching or replacing the template invalidates pending capture results. A provider must advertise video recording separately from preview and photo support. AVFoundation takes silent H.264 samples from the same camera session, encodes an MP4 and bounds the output to 60 seconds and 256 MiB. It does not open a microphone or read back the preview texture. The other providers currently expose preview and still capture; their Record action is disabled.

The WinUI, macOS and Material themes supply the transport and capture templates. The optional control packages publish their own `WINUI_ONLY_CONTROLS` lists to avoid a dependency cycle with `ferroui-controls`. Fluent and Simple supply no templates for them. The existing independent media samples select WinUI so their transport remains available.

ControlCatalog exposes a page for each control. Its three video pages reuse the same video example and its single player. WinUI Gallery has native MediaPlayerElement, CaptureElementPreview and Sound adaptations. The video example uses the shared, attributed Sintel clip and the desktop storage picker. The Sound page plays application-owned PCM cues through the media provider. It records WinUI's Windows-specific spatial audio service as not applicable. The Gallery status describes example coverage; it does not certify every renderer or capture provider.

Run the desktop Gallery from the workspace with:

```sh
cargo run -p winui-gallery --features desktop -- --page=MediaPlayerElement
cargo run -p winui-gallery --features desktop -- --page=CaptureElementPreview
cargo run -p winui-gallery --features desktop -- --page=Sound
```

On macOS the executable embeds the camera usage description. A distributed app bundle must also carry the description in its own Info.plist. On Linux, preview still requires compatible EGL/DMA-BUF support and a working camera portal; no software preview fallback is selected.

Use the portable camera and video-control suites to verify ownership, cancellation, seek and presentation contracts. The AVFoundation encoder fixture supplies generated native samples and reads back the *encoded movie* with an independent `AVAssetReader`, then checks that cancel releases the recording slot. It does not request permission or establish physical-camera acceptance.

```sh
cargo test -p ferroui-media-camera -p ferroui-media-camera-controls -p ferroui-media-video-controls --lib
cargo test -p ferroui-media-camera-apple --lib --features native-lifetime-tests
cargo test -p ferroui-themes-winui --lib media_transport_template
cargo test -p winui-gallery --lib
```

Use the [Windows and Linux handover](MEDIA-CONTROLS-HANDOVER.md) for native acceptance and remaining recorder integration. Physical capture on iOS and Android recording provider support also remain open. The previously recorded #550 verification is evidence for the shared stack on its recorded revisions.
