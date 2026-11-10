# Media controls handover for #561

The branch adds the WinUI controls and examples over the existing portable media stack. Windows and Linux already have native playback, GPU preview, and still-photo providers. Keep those providers and their frame admission rules. The new portable video recording contract is implemented by AVFoundation; Windows and Linux do not advertise recording yet, so their Record action is disabled.

## Windows

Run the existing catalog smoke with ANGLE and D3D11:

```powershell
$env:FERROUI_SMOKE_RENDERING = 'angle'
$env:FERROUI_SMOKE_MEDIA = '1'
cargo run -p control-catalog-desktop
```

Look for `CATALOG_MEDIA_SMOKE_PASS`. With a supported camera available, repeat with `FERROUI_SMOKE_MEDIA_CAMERA=1`, then exercise photo capture through `CameraCaptureUI`. Record whether the camera is physical or synthetic. Check permission denial and navigation away while capture is preparing. The Sound cues use the existing WASAPI path with a PCM-only source. Verify playback, end-of-stream and replay without a video track or texture import.

## Linux

Run the existing catalog smoke with an EGL display that admits DMA-BUF media textures:

```sh
FERROUI_CATALOG_WAYLAND=0 FERROUI_SMOKE_RENDERING=egl FERROUI_SMOKE_MEDIA=1 cargo run -p control-catalog-desktop
```

Repeat with `FERROUI_SMOKE_MEDIA_CAMERA=1` using a working camera portal. Exercise photo capture through `CameraCaptureUI`. If the VM cannot import the negotiated DMA-BUF format or modifier, record the unsupported profile separately from a successful GPU run. Keep the existing portal consent rules.

## Acceptance checks

- Open the MediaPlayerElement, CaptureElementPreview and Sound pages in WinUI Gallery. Play and pause the bundled video, seek, change volume and mute, resize while playing, enter full window, and exit with Escape. Switch sources and navigate away; old frames and playback must retire.
- On the MediaTransportControls catalog page, verify compact layout and button visibility. Check keyboard focus and the changing Play/Pause, Mute/Unmute and full-window names with the host's accessibility inspector.
- Start the camera explicitly, check preview, take a photo, stop, and navigate away. A photo should remain usable after capture stops. Check that a cancelled or superseded operation cannot publish a result.
- Record the renderer, provider, capture source class, outcome, and whether GPU frames retired without CPU preview transfers. Separate successful runs from unsupported profiles and blocked hardware checks. Report failure messages without machine identifiers or local paths.
- Add silent recording to the existing native camera session where the backend supports it. Implement `NativeCameraRecording` on that session and advertise it only once it works. Return bounded, independently owned encoded output; cancel on Stop, detach, or device loss. Use the existing camera samples and the native encoder. Verify that no microphone permission is requested and that the preview import path is unchanged.

The portable media control tests, AVFoundation encoder fixture and native Gallery construction tests have passed locally. This handover leaves Windows and Linux runtime acceptance open.
