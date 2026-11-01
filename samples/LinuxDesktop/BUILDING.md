# Build and run Ferro Desktop

Use a native Linux Rust toolchain (the workspace declares Rust 1.89 or later), a C++17 compiler, pkg-config, fontconfig development files, and the development dependencies for the FerroUI X11 and Skia backends. The header fetch uses Bash, curl, GNU tar, sha256sum and flock. Running the accelerated session requires an X server, a Vulkan loader, and a driver with Vulkan 1.1 and X11 presentation support.

From the repository root, build a release binary or start the shell inside your current X11 desktop.

```sh
samples/LinuxDesktop/build.sh
samples/LinuxDesktop/build.sh run
```

The wrapper selects the Linux Graphite and Vulkan build of Skia for the desktop's dependency graph. It verifies the source-header archive for the locked rust-skia release and caches the required Skia, Vulkan and skcms headers under `$XDG_CACHE_HOME/ferroui/skia` (default `~/.cache/ferroui/skia`). Builds sharing a cache can start concurrently. To use an existing matching checkout without fetching the header archive, set `FERROUI_SKIA_HEADERS` to the source root for rust-skia/skia tag `m153-0.101.2`. The checkout must contain `include/` and `modules/skcms/`. For a fully offline build, Cargo dependencies and the corresponding native rust-skia binary must also be cached.

```sh
FERROUI_SKIA_HEADERS=/opt/skia-m153-0.101.2 samples/LinuxDesktop/build.sh check --offline
samples/LinuxDesktop/build.sh build --profile dev
samples/LinuxDesktop/build.sh test
```

## Run as a login session

Install Openbox 3.6 or later, Bash, D-Bus, and your distribution's Xorg display-manager support. Install the binary produced by the release build, then select **Ferro Desktop (X11)** in the login screen.

```sh
sudo samples/LinuxDesktop/session/install.sh target/release/ferro-desktop
```

The installer writes the binary and supervisor to `$PREFIX/bin` (default `/usr/local/bin`), the Openbox configuration to `$PREFIX/share/ferro-desktop`, and the login entry to `$XSESSIONS_DIR` (default `/usr/share/xsessions`). The display manager must have `$PREFIX/bin` in its executable search path. Use `DESTDIR` to stage all files for packaging. Set `$XDG_CONFIG_HOME/ferro-desktop/openbox.xml` to customize the window-manager bindings.

| Action | Login-session shortcut |
| --- | --- |
| Reveal the desktop and open the launcher, or return to the applications | Super+Space or Super+D |
| Switch to a workspace | Super+1 through Super+4 |
| Move the focused window to a workspace | Super+Shift+1 through Super+Shift+4 |
| Cycle windows | Alt+Tab or Alt+Shift+Tab |
| Toggle maximize / minimize the focused window | Super+Up / Super+Down |
| Close the focused window | Alt+F4 |
| End the session with confirmation | Ctrl+Alt+Delete |

## Test the session adapter

The native integration test creates a foreign X client and drives focus, minimization, title changes, workspace moves, close negotiation and removal through Openbox. Use an isolated X server (Xvfb and xauth required).

```sh
cd samples/LinuxDesktop
xvfb-run -a sh -ec '
    openbox --sm-disable --config-file session/openbox.xml &
    wm=$!
    trap "kill $wm 2>/dev/null || :" EXIT
    ./build.sh test -- --ignored real_client_lifecycle
'
```
