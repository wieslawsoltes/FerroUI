# The VNC server of the headless platform

Upstream project: `src/Headless/Avalonia.Headless.Vnc` (three files). Port: the crate `ferroui-headless-vnc` at `src/Headless/FerroUI.Headless.Vnc`, which until this port was an empty placeholder that reserved the name (`docs/release.md`, section 2.2).

## 1. What upstream does

`StartWithHeadlessVncPlatform(host, port, [password,] args, shutdownMode)` starts an application on the headless platform with drawing by the real rendering backend (`UseHeadlessDrawing = false`, framebuffers of `Bgra8888`), binds a TCP listener (the loopback address unless a host is given) and runs the classic desktop lifetime. When the lifetime starts, an asynchronous loop on the UI thread accepts the clients. For each one it creates a `VncServerSession` of the .NET library RemoteViewing (package `Quamotion.RemoteViewing` 1.1.211), with password authentication when a password was given, and gives it a `HeadlessVncFramebufferSource` for the main window.

`HeadlessVncFramebufferSource` implements `IVncFramebufferSource` of RemoteViewing:

- `Capture` copies the last rendered frame of the window (`GetLastRenderedFrame`) into a `VncFramebuffer` of the pixel format `RGB32`, replaced when the size of the frame changes;
- `SetDesktopSize` sets the width and the height of the window (the client asked for another size);
- the pointer events of the session become `MouseMove`, `MouseDown`, `MouseUp` and `MouseWheel` of the headless window; the key events become `KeyPress`, `KeyRelease` and, on a release without Control or Alt, `KeyTextInput` with the text of the key symbol (`TranslateKey`, a table of X11 key symbols); Shift, Control and Alt are kept as the modifier state. The input is posted to the UI thread at input priority.

`AvaloniaVncLogger` is the `ILogger` of `Microsoft.Extensions.Logging` the sessions log to; it writes to the logger of the framework, area `VncPlatform`.

The ControlCatalog desktop sample starts with the server when it is given `--vnc` (port 5901, `ShutdownMode.OnMainWindowClose`). Upstream has no tests of the project.

## 2. What the port does

| Upstream file | Rust file | Notes |
|---|---|---|
| `AvaloniaVncLogger.cs` | `ferro_vnc_logger.rs` | `FerroVncLogger`, an `ILogger` of `remote_viewing::logging` |
| `HeadlessVncFramebufferSource.cs` | `headless_vnc_framebuffer_source.rs` | `HeadlessVncFramebufferSource`, an `IVncFramebufferSource` |
| `HeadlessVncPlatformExtensions.cs` | `headless_vnc_platform_extensions.rs` | the trait `HeadlessVncPlatformExtensions` of `AppBuilder`: `start_with_headless_vnc_platform` and `start_with_headless_vnc_platform_with_password` (the second overload) |

Every type, member and step of the three files is ported. The differences, each with a comment at its site and a row in `DEVIATIONS.md` (section "VNC server of the headless platform"):

- **The accept loop.** A thread of the server waits for the clients (`TcpListener::accept`) and posts each connection to the UI thread, where its session is created as upstream creates it; the loop starts on `Startup` and waits 100 ms after each client, as upstream. Upstream awaits `AcceptTcpClientAsync` on the UI thread; the port has no asynchronous sockets.
- **The capture.** RemoteViewing calls `Capture` on the threads of the session. Upstream reads the last frame of the window there; the window of the port belongs to the UI thread, so `capture` runs on the UI thread (a synchronous invoke) and the session waits for it. When the UI thread has shut down, the framebuffer of before is returned.
- **The order of the input.** Upstream posts each input event on its own. The input extensions of a headless window run the jobs of the dispatcher before they deliver an event, so an event posted while another is being delivered overtakes it: a release that follows a press closely (as a program, not a person, sends them) is delivered before the press, and the click is lost. The port queues the events in the source and each post delivers the queue in order; a delivery that starts inside another leaves the queue to it. A person at a viewer sees no difference.
- **Signatures.** The port is a `u16`; the shutdown mode has no default value; an address that cannot be parsed or bound panics (the exceptions of `IPAddress.Parse` and `TcpListener.Start`).

The ControlCatalog desktop host (`samples/ControlCatalog.Desktop/program.rs`) takes `--vnc` as upstream: `start_with_headless_vnc_platform(None, 5901, &args, ShutdownMode::OnMainWindowClose)`.

## 3. The RFB server: the choice

The original is built on RemoteViewing. The port needs the server side of RFB (RFC 6143) with the shape the original uses: a session per connection with a framebuffer source that is asked for a frame when the client wants an update (a pull model), a request to resize the desktop, key and pointer events, and optional VNC password authentication.

The Rust crates that implement an RFB server were looked at on 2026-10-15:

| Crate | Licence | State | Fit |
|---|---|---|---|
| `rustvncserver` 2.2.1 | Apache-2.0 | active, one author | push model (the application writes into a framebuffer of the server), no desktop resize request; Tokio multi-thread runtime, `rand`, `des`, `flate2`, `rfb-encodings` |
| `rfb` 0.1.0 (Oxide Computer) | MPL-2.0 | one release, 2022 | asynchronous (Tokio with all features), `image` 0.24, `env_logger` as a normal dependency; MPL-2.0 is file-level copyleft |
| `rfb2` 0.1.2 | MPL-2.0 | a fork of `rfb` | as `rfb` |
| `vncrs` 0.1.8 | MIT | Windows only | captures the Windows desktop (`windows-capture`, `enigo`) |
| `vnc` 0.4.0 | MIT OR Apache-2.0 | last release 2017 | client and proxy, no server; `log` 0.3, `byteorder` 0.5, `flate2` 0.2 |
| `vncserver`, `libvnc` | GPL (LibVNCServer) | bindings | C library under the GPL |

None fits: the one maintained server with a compatible licence has a different model (a framebuffer owned by the server and pushed into, no resize request) and brings an asynchronous runtime into a platform that has none. So the parts of RemoteViewing the original uses are ported, under `remote_viewing/` of the crate:

- the server session and what it needs: `VncServerSession`, its options, events and interface, the framebuffer cache that finds the changed lines, the encoders (raw and Tight with basic zlib compression and JPEG compression), the pixel formats, framebuffers, rectangles, key symbols, the password challenge, the stream of the protocol, `AutoClear` and `PeriodicThread`;
- not ported: the client, `VncServer` (a listener of the library the original does not use), `ZlibEncoder` (the session does not use it), the hosting extensions, the VMware movie writer, and the overloads of `VncPixelFormat.Copy` over raw pointers;
- `remote_viewing/logging.rs` stands for the part of `Microsoft.Extensions.Logging` the library logs through.

RemoteViewing is under the BSD-2-Clause licence (Copyright (c) 2013 James F. Bellinger), not MIT: its notice is kept in `src/Headless/FerroUI.Headless.Vnc/NOTICE.md` and named in the `NOTICE.md` of the repository. The ported files name their source in their header.

Dependencies of the RFB server, all MIT OR Apache-2.0 and without native code: `flate2` with its Rust backend (zlib of the Tight encoding, in place of SharpCompress), `jpeg-encoder` (JPEG of the Tight encoding, in place of libjpeg-turbo, which the library calls through a wrapper; the crate was already a dependency of the Vello backend), `des` (the DES of the VNC password challenge, in place of the provider of .NET) and `getrandom` (the random challenge).

### 3.1 Where the port of RemoteViewing differs from it

These are corrections of the library, made where its behaviour does not follow the protocol; each file says so at its top.

- `VncPixelFormat::copy` converts between pixel formats of true colour, as its documentation says. The library copies between equal formats only and leaves the target untouched otherwise, so a viewer that asks for a pixel format of its own received black pixels.
- The Tight encoder sends data of fewer than 12 bytes (after the pixels are reduced to three bytes) as it is, without a length, as the protocol says; the library writes a length in front and compares the size before the reduction.
- `TightCompressionControl::RESET_STREAM_3` is the bit of the protocol (`0b1000`); the library has `0b10100`. Nothing uses it.
- The framebuffer cache sends the runs of changed lines of an incremental update at their place; the library adds the top of the requested region twice and makes each run one line longer, which only matches for a request from the top of the framebuffer. The cache also clips a request to a framebuffer that became smaller since.
- The extended desktop size rectangle carries the status of a resize request in its y-position, as the protocol says; the library always sends 0 (success).
- The statistics of the encoders are logged at the end of a session only when it has a logger; the library logs them through a logger it does not check.
- The monitors of .NET are a reentrant lock (`ReentrantLock`) where the session takes a lock it holds; the events are `VncEvent` values (`add` and `remove` for `+=` and `-=`); a panic of an event handler or of the framebuffer source ends the session as an exception of them ends it in the library.

## 4. How it is tested

`cargo test -p ferroui-headless-vnc` (with `CARGO_TARGET_DIR` and `TMPDIR` on the external volume):

- 34 unit tests of the RFB server: the encoding of the pixel format and its conversion, rectangles, the stream of the protocol over a socket pair, the password challenge (DES, the bit reversal, longer passwords), the arguments of the password event, the Tight encoder (the compact lengths, basic compression decoded with zlib, small data, a JPEG image), the periodic thread, the reentrant lock, the logging extensions, the key table of the framebuffer source, the levels of the logger; and sessions driven over socket pairs: a client without authentication gets the server init message and a full update whose pixels are the framebuffer, then an incremental update of the changed line only; a client with the right password is accepted and one with another is refused; a client of protocol 3.3 is refused with the reason; key, pointer and clipboard messages raise the events.
- `tests/headless_vnc_server.rs`: an application with a button and a text box starts with `start_with_headless_vnc_platform` on a free loopback port, rendering with Skia; the client of `tests/support/rfb_client.rs` (written for the test: version 3.8, security type None, raw encoding, the desktop size pseudo-encodings, key and pointer events) connects; the server init message has the size of the window (320 by 200) and the pixel format `RGB32`; a full update is compared pixel by pixel with the last frame the window rendered (`get_last_rendered_frame`); a click at the centre of the button raises its click event once; a click on the text box and the keys `a` and `b` make its text `ab`; an incremental update brings the changed pixels; closing the main window ends the application with exit code 0. The frames the client received are written to `$CARGO_TARGET_TMPDIR/headless-vnc/`.
- `snapshot_of_a_running_server` (ignored) takes a picture of a running server through the same client: `FERROUI_VNC_ADDRESS`, `FERROUI_VNC_SNAPSHOT`, and `FERROUI_VNC_CLICK=x,y` to click first.

The ControlCatalog was run on macOS with `--vnc` (headless, no window on the screen) and pictured through the test client; the pictures are in `<workspace>/ferroui-screenshots/headless-vnc/` of the machine of the port (section 5).

Not a test of this crate: one test of `ferroui-headless` (`rendering_tests::should_render_to_a_compositor_snapshot_capture`, the `per_assembly` variant) failed once in three runs on the branch, which does not change that crate; it is intermittent and unrelated to the server.

## 5. What was verified, and what was not

Verified on macOS (Apple silicon), 2026-10-15: the tests above (three runs in a row, all green); `control-catalog-desktop --vnc` runs without a window on the screen and serves the catalog on port 5901. Three connections of the test client, one after the other, each a session of its own:

- `01-controlcatalog.png`: the home page, 1100 by 800 (the size the main window asks for);
- `02-controlcatalog-buttons-clicked.png`: after a click on the card "Buttons" of the home page, the Buttons page is shown and the navigation expanded;
- `03-controlcatalog-text-clicked.png`: after a click on "Text" in the navigation, the Text page.

The pictures are the framebuffer updates the client received, decoded into PNG files; no picture of the screen of the machine was taken.

Not verified:

- real VNC viewers (TigerVNC, RealVNC, Remmina, noVNC through a WebSocket proxy). The macOS Screen Sharing application speaks the version 3.889 of Apple and is refused by the version check of the library, as upstream refuses it;
- the Tight encoding and its JPEG compression against a viewer (only decoded by the unit tests: zlib inflated, the JPEG marker checked);
- password authentication against a viewer (only against the DES of the port itself);
- the server on Linux and Windows (the crate has no platform code of its own). The unit tests that drive a session or the stream run over Unix socket pairs and are compiled on Unix only; the test of `tests/` uses TCP and runs everywhere.

## 6. Use

```sh
cargo run -p control-catalog-desktop -- --vnc
# then a VNC viewer at 127.0.0.1:5901 (display :1), or the client of the tests:
FERROUI_VNC_SNAPSHOT=/tmp/catalog.png cargo test -p ferroui-headless-vnc --test headless_vnc_server -- --ignored
```
