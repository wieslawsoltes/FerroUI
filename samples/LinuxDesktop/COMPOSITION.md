# FerroUI desktop composition

The FerroUI compositor mode combines the desktop and independent native Wayland windows in one retained FerroUI visual tree. Smithay implements the child Wayland display, xdg-shell roles and seat. FerroUI owns client placement, stacking, workspaces, hit testing and composition. The Graphite presenter sends the finished output to a host window. In the dedicated login, Sway supplies the physical seat, fullscreen output window and secure lock surface.

## Run it

Start a composited desktop in a window on your existing Wayland or X11 desktop. The shell prints its child socket name when ready. Supply an unused name to make it easy to connect an independent client. The socket lives in your existing `XDG_RUNTIME_DIR`.

```sh
samples/LinuxDesktop/build.sh run -- --compositor=ferro --socket=ferro-lab
```

In another terminal, connect applications to the new display.

```sh
env -u WAYLAND_SOCKET -u DISPLAY -u SWAYSOCK WAYLAND_DISPLAY=ferro-lab foot
env -u WAYLAND_SOCKET -u DISPLAY -u SWAYSOCK WAYLAND_DISPLAY=ferro-lab GDK_BACKEND=wayland gtk3-demo
```

The shell launcher uses a GIO launch context with the child display. Directly spawned applications receive their own environment. Existing single-instance applications on a shared D-Bus session can still route activation to a running instance. For a separate activation bus and the login keybindings, install the sample and run `ferro-desktop-wayland --ferro --nested`. To use the dedicated login, select **Ferro Desktop (FerroUI Compositor)** in the display manager. The supervisor sends the child display to D-Bus-activated applications in this session. It also routes **Super+Return** to a terminal inside the composed desktop.

## What gets composed

| Stage | Implementation |
| --- | --- |
| Client commit | The protocol worker reads ARGB8888 or XRGB8888 shared-memory buffers into owned premultiplied BGRA snapshots. It checks offsets, strides and lengths before copying and releases the client buffer after import. |
| Surface tree | Toplevels, synchronized subsurfaces and popups retain their protocol order and positioning. Hidden workspaces and minimized windows stay in the window model. Only visible surfaces enter the output scene. |
| FerroUI visuals | Each client surface has a retained `CompositionSurfaceVisual` and `CompositionDrawingSurface`. A new pixel revision updates the bitmap. Moving or raising the window changes visual properties or child order. |
| Presentation | The UI dispatcher coalesces commits. The renderer reports completion of the FerroUI composition batch to the protocol worker, which completes the matching Wayland frame callbacks. Skia Graphite on Vulkan is the default output renderer. `--software` selects raster explicitly. |
| Shell | The wallpaper sits below the clients. The taskbar, Dock, menus, launcher and passive notification banners sit above them. Fullscreen clients use the whole output. Appearance changes keep the client connections and visual layer alive. |

The input seat targets the topmost eligible client surface, including its popups and subsurfaces. On a native Wayland host, the seat receives the parent’s current XKB map, layout group, modifier state and evdev codes. The X11-hosted preview uses the default evdev layout. Child applications exchange clipboard selections over the private display. Use **Alt+primary drag** to move and **Alt+secondary drag** to resize a client. The task overview exposes activate, close, minimize, maximize, restore, snap, fullscreen and workspace operations. **Super+1–9** switches to an existing workspace and **Super+Shift+1–9** moves the focused client. **Super+Left/Right**, **Super+Up/Down** and **Super+F** control snap, maximize/minimize and fullscreen. A windowed host may reserve system shortcuts. Its shell controls provide the same operations. With a child focused, its own shortcuts, including Ctrl+W, go to that application.

## WinUI appearance

Open Appearance in the Windows profile to select Light, Dark, High Contrast or Compact density. The shell uses FerroUI’s compiled WinUI theme dictionaries, semantic brushes, controls and flyout composition resources. macOS and Material remain independent profiles. GTK and Qt clients draw their content and client-side decorations with their own toolkits. See [appearance profiles and extension points](APPEARANCE.md).

## Scope

The child compositor exposes one logical output, native xdg-shell clients and shared-memory buffer import. Direct DMA-BUF import, DRM/KMS scanout, Xwayland, touch/tablet seats and a child screen-capture portal remain outside this mode. The Sway-managed login remains available for native multi-output management, Xwayland and portal integration. See [Wayland sessions](WAYLAND.md).

The integration tests use Sway’s headless backend, GTK 3, PyQt5/Qt Wayland, Pillow, Weston clients, grim, wtype, jq, ImageMagick, a C compiler, libwayland development files and wayland-scanner. The pointer fixture uses the wlr protocol XML from the Cargo registry or the installed wlr-protocols package. Run the tests with a Graphite development build.

```sh
samples/LinuxDesktop/build.sh build --profile dev
samples/LinuxDesktop/tests/ferro-compositor.sh target/debug/ferro-desktop /tmp/ferro-compositor
samples/LinuxDesktop/tests/wayland-session.sh --ferro target/debug/ferro-desktop /tmp/ferro-composed-login
```
