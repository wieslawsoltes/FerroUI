# Wayland sessions

Ferro Desktop uses the native FerroUI Wayland backend and Skia. In a managed login session Sway is the wlroots compositor. It owns DRM/KMS, input seats, output configuration, stacking and workspaces. The desktop shell owns the layer surfaces, launcher, settings and notification UI. Independent Wayland applications connect to the same compositor. Xwayland is optional for legacy applications.

Changing appearance updates the wallpaper, panels and controls on all outputs. Sway also recolors the titlebars and borders it draws for existing clients. Applications with client-side decorations draw their own titlebars.

## Install or run

Build the release with `samples/LinuxDesktop/build.sh`. Install Sway, the D-Bus session tools, Mesa or another Vulkan driver, and xkbcommon. For screen sharing, install PipeWire, a PipeWire session manager, xdg-desktop-portal, xdg-desktop-portal-wlr and xdg-desktop-portal-gtk. Install swaylock for the lock shortcut and swayidle for automatic locking. A graphical login needs a working logind/seatd seat and access to the GPU.

```sh
sudo samples/LinuxDesktop/session/install.sh target/release/ferro-desktop
```

Select **Ferro Desktop (Wayland)** in the display manager. The installer also provides the X11 entry. Packaging can stage both with `DESTDIR` and override `PREFIX`, `WAYLAND_SESSIONS_DIR` or `XSESSIONS_DIR`.

To open a normal application window inside an existing Wayland desktop, run the following command.

```sh
samples/LinuxDesktop/build.sh run -- --platform=wayland
```

For an independent development session inside an existing compositor, use the installed `ferro-desktop-wayland --nested`. It creates the first nested Wayland output after Sway starts its event loop, which also works on software-rendered Sway and Weston hosts. It starts a new Sway instance and a private session bus. For a direct seat login the supervisor uses the login bus or starts a private bus when there is none. It obtains the real Wayland/Xwayland and IPC addresses from Sway, exports them for D-Bus activation, and waits for both shell and compositor. A direct login clears inherited display addresses before acquiring the seat. A native-only session exports no parent X display to its shell. A normal session ending or a component exiting tears down the other component.

## Session policy

The default Sway policy is in [sway.conf](session/sway.conf). Copy it to `$XDG_CONFIG_HOME/ferro-desktop/sway.conf` (default `~/.config/ferro-desktop/sway.conf`) to adjust bindings, output scale/rotation/arrangement, workspace routing, keyboard layouts and input policy. `FERRO_DESKTOP_SWAY_CONFIG` selects an alternate config explicitly. Relative includes resolve next to their source file. Set `FERRO_DESKTOP_XWAYLAND=0` for a native-only login or `1` for a login with Xwayland enabled. The default enables Xwayland when its server is installed.

| Shortcut | Action |
| --- | --- |
| Super+Space, Super+A, Super+N, Super+Tab | Launcher, Quick Settings, notifications, workspace overview |
| Super+1…4, Super+Shift+1…4 | Switch workspace or move a native client there |
| Super+arrow, Super+Shift+arrow | Focus or move in the tiling layout |
| Super+F, Super+Shift+Space | Toggle fullscreen or floating |
| Super+Return, Alt+F4 | Launch a terminal or close the focused client |
| Super+L | Lock the session with swaylock |
| Super+Shift+C, Super+Shift+E | Reload Sway configuration or confirm logout |

With swaylock and swayidle installed, the default login policy locks after ten minutes of inactivity, before suspend, and on a logind session lock event. The compositor confirms the lock before swayidle releases its sleep inhibitor. The nested launcher skips the host's idle and suspend hooks. Customize the `exec ferro-desktop-wayland --idle` line in the Sway policy to adjust lock behavior.

The normal Wayland connection may use an inherited file descriptor. Managed actions resolve the socket pathname from its peer, so a conflicting `WAYLAND_DISPLAY` cannot redirect a command. An anonymous socket pair supports application rendering but cannot identify a named session endpoint.

Global shell actions use a versioned D-Bus interface on a privately owned service name derived from the Wayland socket's identity. It accepts fixed shell commands and reads back the current scene. A second managed shell cannot replace the first owner. Normal application previews leave the endpoint unclaimed. The [native window and workspace reference](WAYLAND-WINDOWS.md) explains focus, tiling, scratchpad and client lifecycle behavior.

### Portals

The login exports `XDG_CURRENT_DESKTOP=FerroDesktop:sway`. Its desktop-specific [portal policy](session/ferrodesktop-portals.conf) chooses the wlr backend for ScreenCast and Screenshot and GTK for its other supported interfaces, including FileChooser. The compositor startup handoff publishes the real `WAYLAND_DISPLAY` and `SWAYSOCK` to the activation environment. A user's existing portal override retains priority. See the [portal configuration reference](https://flatpak.github.io/xdg-desktop-portal/docs/portals.conf.html) and [wlroots backend](https://github.com/emersion/xdg-desktop-portal-wlr).

Screen sharing depends on the compositor's capture protocols, available buffer formats, portal version and PipeWire services. On Ubuntu 24.04, xdg-desktop-portal-wlr 0.7.1 requires a DMA-BUF format from version 3 of the screencopy protocol. The stock portal negotiates sources and supplies screenshots in the headless test. Streaming frames with this release additionally needs a compositor/driver that supplies DMA-BUF. [Upstream 0.7.1 implementation](https://github.com/emersion/xdg-desktop-portal-wlr/blob/v0.7.1/src/screencast/screencast.c#L190-L205)

## Reproduce the native validation

The session test starts the installed policy and shell on a fresh headless Sway with two outputs, including one at fractional scale. It uses `foot` and `gtk3-demo` as real independent xdg-shell clients, `wtype` for native virtual-keyboard input, `wl-clipboard` for cross-client selection transfer and `grim` for compositor frames. It verifies the shell command endpoint, workspace movement and focus restoration. The native run disables Xwayland. The mixed run additionally maps an X11 client through the compositor's Xwayland bridge.

```sh
samples/LinuxDesktop/tests/wayland-session.sh target/release/ferro-desktop /tmp/ferro-wayland
samples/LinuxDesktop/tests/wayland-session.sh --xwayland target/release/ferro-desktop /tmp/ferro-mixed
```

Optional fixtures exercise a dedicated nested desktop with its own display and bus, output hotplug, independent Qt Widgets and GTK clients on screen together, the compositor-confirmed screen lock and the public portal. The private session grants its test identity permission to take a screenshot. It creates a screencast session and selects a monitor through the public ScreenCast API. User permission stores and sharing preferences stay on the login bus.

```sh
FERRO_TEST_NESTED=1 FERRO_TEST_OUTPUT_HOTPLUG=1 FERRO_TEST_QT=1 FERRO_TEST_LOCK=1 FERRO_TEST_PORTALS=1 \
  samples/LinuxDesktop/tests/wayland-session.sh target/release/ferro-desktop /tmp/ferro-wayland
```

For a full stream, `FERRO_TEST_SCREENCAST=1` calls Start and OpenPipeWireRemote and feeds the returned file descriptor and node ID to GStreamer's pipewiresrc. It saves the first decoded frame as `portal-capture.png`. The test uses a pixman compositor, so the portal backend must accept shared-memory screencopy buffers. The upstream 0.5.0 release supports this. You can pass a separately built test binary without changing the installed portal. [Upstream 0.5.0 source](https://github.com/emersion/xdg-desktop-portal-wlr/tree/v0.5.0)

```sh
FERRO_TEST_SCREENCAST=1 FERRO_TEST_WLR_PORTAL=/absolute/path/to/xdg-desktop-portal-wlr \
  samples/LinuxDesktop/tests/wayland-session.sh target/release/ferro-desktop /tmp/ferro-stream
```

The optional portal tests require `pipewire`, `wireplumber`, `xdg-desktop-portal`, `xdg-desktop-portal-wlr`, `xdg-desktop-portal-gtk` and `python3-gi`. The full stream also needs the Ubuntu packages `gstreamer1.0-tools`, `gstreamer1.0-pipewire`, `gstreamer1.0-plugins-base` and `gstreamer1.0-plugins-good`. The lock test needs `swaylock`. The Qt fixture uses Ubuntu packages `python3-pyqt5` and `qtwayland5`.

The test dependencies include ImageMagick, Sway and swaymsg, jq, wayland-utils, foot, gtk-3-examples, wtype, wl-clipboard, grim and dbus-daemon. Mixed mode also needs Xwayland and xmessage. When invoked as root the test stages a temporary executable and runs Sway under an unprivileged account.


### Measure the launcher and idle shell

```sh
FERRO_TEST_BENCHMARK=1 samples/LinuxDesktop/tests/wayland-session.sh target/release/ferro-desktop /tmp/ferro-bench
```

On the empty first workspace, the fixture warms up two complete launcher toggles. It then times ten opens and closes triggered by `--show=applications` using a monotonic clock. Each timer ends when a quarter-resolution `grim` capture matches the settled destination at 90% of distinguishing grid points. This includes launching the CLI, D-Bus delivery, rendering, compositor presentation and pixel readback. With the desktop visible again, the fixture samples the managed shell's Linux process CPU ticks over five idle seconds. `wayland-benchmark.json` contains every observed latency, both medians, the idle interval, the tick frequency and the software renderer configuration.
