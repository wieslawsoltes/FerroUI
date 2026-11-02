# Media in ControlCatalog

The main catalog has **Video**, **Camera** and **QR codes** pages under
**Media & Graphics**. They use the framework controls and the existing catalog
navigation, styling and accessibility names. Creating a page starts no media
operation. Navigation away retires playback/capture and rejects stale results.

- Video loads a bundled 12-second Sintel trailer excerpt or an entered file path or
  supported URL. Autoplay stays off. The optional transport controls demonstrate
  play/pause, stop, seeking, volume and mute. Fit/fill can be changed separately.
- Camera discovery does not request consent or acquire capture hardware.
  Starting capture requests permission and a preferred 640×480/30 fps profile;
  the page reports actual negotiated settings. Photos show independently owned
  decoded output and its native-still/preview-snapshot provenance. Only one photo
  request is issued at a time, and the latest image stays in memory.
- The QR codes page decodes a bundled two-symbol image with the optional Rust decoder.
  Native hosts run this static-image work off the UI thread with one outstanding
  image job. Apple desktop also supports explicitly started live Vision analysis
  through the bounded QR analysis gate. Results are text, never automatically
  opened links. “Last detected” is history, not a geometric overlay.

## Host bindings

The shared catalog accepts provider factories through `media_samples::install`.
The **desktop host installs them**: Metal on macOS, ANGLE/D3D11 on Windows and
admitted EGL/DMA-BUF on Linux. Unsupported rendering profiles produce visible
errors. The catalog does not select experimental macOS CGL media.

The browser host also installs native playback/capture providers for the
single-threaded, page-owned Skia WebGL profile. Worker media bridges remain
deferred: a threaded module or unsupported renderer reports unavailability.
Offline QR decoding works in the browser; live native recognition is not
installed there. The iOS and Android catalog entry points do not yet install
playback/capture factories. Their pages report unavailability and disable live
QR start; their separate PortableMedia.Mobile integration sample retains its
platform bindings. Desktop checks do not claim mobile/browser runtime support.

## Standard video and browser download behavior

The Sintel excerpt is 423,728 bytes (about 414 KiB), with H.264 video and AAC
audio. Blender Foundation publishes the trailer under CC BY 3.0, permitting
sharing and adaptation with attribution. The player displays the author,
project and license URLs, and a trimming/re-encoding notice. The source,
license text, hashes and reproduction script are in
[the media directory](../../samples/ControlCatalog/Media/README.md).

The browser build copies the MP4 to `media/sintel-sample.mp4`. It is excluded
from WebAssembly and from the catalog asset manifest, startup preloads and
idle prefetch. Opening the catalog or the Video page does not request it.
Choosing **Load bundled video** opens the same-origin URL through the browser's
media element; no remote video service is needed at runtime. Native builds
embed it for use without network access.

The desktop build embeds a macOS camera usage description in its executable.
A packaged `.app` must also carry that description in its bundle Info.plist and
the appropriate signing/entitlements. Discovery does not test permission grants,
physical preview or photos. Windows/Linux catalog bindings require native host
compilation and execution; they were not verified by the macOS run.

## Reproduce the desktop check

```sh
cargo build -p control-catalog-desktop
FERROUI_SMOKE_RENDERING=metal FERROUI_SMOKE_MEDIA=1 \
  target/debug/control-catalog-desktop
```

`FERROUI_SMOKE_MEDIA=1` is an explicit development smoke. It mutes and plays the
Sintel clip, checks actual GPU imports and zero CPU preview transfers, navigates
away and waits for zero retained frames/GPU reads/quarantine, discovers cameras
without starting capture, decodes both bundled QR symbols, then closes its own
window. A failure exits nonzero; success prints `CATALOG_MEDIA_SMOKE_PASS`.
It does not start a camera or prompt for permission. Do not combine it with the
catalog's other automatic navigation modes.

```sh
cargo test -p control-catalog --lib bundled_qr_image_decodes_two_independent_symbols
FERROUI_SMOKE_RENDERING=metal \
  FERROUI_SMOKE_SCREENSHOTS=/path/outside/repository/catalog-images \
  FERROUI_SMOKE_SCREENSHOT_PAGES='Video,Camera,QR codes' \
  target/debug/control-catalog-desktop
```

The fixture test checks both expected payloads. Screenshots exercise the idle
pages without starting capture. The native smoke proves playback rendering and
retirement separately; idle screenshots alone are not that evidence.

## Reproduce the browser check

Activate the repository's Emscripten/wasm-bindgen toolchain, then:

```sh
scripts/build-browser.sh control-catalog-browser --debug
FERROUI_BROWSER_ANGLE=metal \
  node scripts/browser/tests/catalog_media.test.mjs target/browser/control-catalog-browser
```

Serve the assembled site with MP4 MIME support (`video/mp4`) and normal static
asset handling. The example test server already does this. The standard
single-threaded browser build supports media; for a combined site, select
`?Threads=false&RenderThread=false&RenderingMode=WebGL2`.

The check uses real pointer input, native HTML playback and Skia/WebGL. It
asserts that the served MP4 matches the source, is absent from WebAssembly and
the catalog asset manifest, and is not requested during startup, ordinary idle
prefetch or Video navigation. Only Load opens its same-origin URL. Autoplay
remains off and Play reaches native playback without overriding browser autoplay
policy. It checks GPU imports, zero CPU preview transfers and full unload
retirement. Camera checks use Chrome's fake device and an explicit test-only
permission grant, then capture/decode a photo, stop capture and decode both QR
symbols. They do not acquire a physical camera.

## Observed catalog results (2026-11-02)

On the Apple M4 development Mac, macOS 27.0.1:

- Desktop build passes. `CATALOG_MEDIA_SMOKE_PASS` confirms Metal presentation,
  paused first-frame/autoplay behavior, actual playback, navigation retirement,
  non-acquiring discovery (zero devices, `NotDetermined`) and both QR payloads.
- The QR fixture unit test passes with both independently expected strings.
- Browser script type checking, lint, bundling and the Emscripten catalog build
  pass. Chrome with ANGLE/Metal prints `CATALOG_BROWSER_MEDIA_PASS`, including
  the lazy-asset, playback, fake-camera/photo/retirement and QR checks above.
- Idle framework screenshots of all three pages and a real browser playback
  screenshot were inspected. These are sample checks, not full accessibility or
  physical camera acceptance.

The first Apple low-quality asset export was rejected as `UnsupportedColor`.
The shipped FFmpeg encode explicitly converts to Rec.709 and passes the same
admission policy; frame admission was not relaxed to accept it. The earlier
failed browser camera check used an invalid test permission name; the passing
test uses the existing harness's `camera` permission helper.

Logs and screenshots are outside Git in
`/Volumes/SSD/build-cache/ferroui-portable-media-evidence/catalog-final-2026-11-02/`.
No camera captures or build products are committed.

## Integration status (2026-11-02)

The desktop branch is not ready for an unconditional fully supported media
release. A read-only merge check against the fetched `origin/main` found content
conflicts in `gpu/graphite/graphite_gr_context.rs` and `gpu/mod.rs` under
`src/Skia/FerroUI.Skia`. These need resolution and a new integrated build before
merging. The branch had 35 unique commits and upstream main had 20 at that check,
before adding the catalog commit (`origin/main` was `df3e12ad7`). No main-branch
merge was performed.

The default Metal profile has its own passing
[verification](MACOS-VERIFICATION.md). Experimental
[CGL](MACOS-CGL-VERIFICATION.md) still fails its reset-memory gate. Positive Linux
DMA-BUF verification and the other profile-specific hardware/release gates remain
as recorded in the [handover](PLATFORM-HANDOVER.md) and platform verification
notes. The workspace Skia source correction also needs an upstream/published
dependency solution before independently published crates give downstream users
the same correction; see
[the dependency boundary](../../external/rust-skia-bindings/FERROUI-PATCH.md).

An experimental integration merge is a separate decision from declaring those
release gates complete. Catalog pages do not close missing platform evidence.
