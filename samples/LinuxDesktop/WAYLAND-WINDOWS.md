# Window and workspace management on Wayland

The managed Ferro Desktop session uses Sway as its Wayland compositor. The workspace overview and panel share the same FerroUI model as the X11 session. Native Wayland and XWayland applications appear together. Shell layer surfaces are outside the client list.

Sway sends changes to a dedicated IPC subscription. The shell subscribes before taking its initial snapshot so a client opening during startup is included. The listener sleeps while idle. It merges title and focus events without requesting the client tree and coalesces structural changes before dispatching one pending update to the FerroUI thread. Drawing the panel or opening the overview reads the cached model.

The IPC backend checks that the Unix socket peer also owns the active Wayland display. This prevents an inherited `SWAYSOCK` from directing commands to an outer compositor in a nested session. Window commands use compositor container IDs. Workspace commands use unique numeric selectors where available. A named workspace whose name cannot be encoded safely remains visible, and a command targeting it returns failure.

## Desktop actions

| Action | Compositor behavior |
| --- | --- |
| Activate or close | Focuses or closes the selected client. Activating a hidden client reveals it from the scratchpad. |
| Minimize | Moves the selected client to Sway’s scratchpad. Hidden scratchpad clients stay accessible in the overview. |
| Fullscreen | Toggles Sway fullscreen on the client. |
| Maximize and restore | Fits the floating window to its workspace. Restore returns it to its previous floating frame or hands a previously tiled client back to the tiling layout. |
| Snap left or right | Requests half the workspace in logical coordinates. The right side receives the extra pixel on odd widths. The compositor and application apply their size constraints. |
| Switch, add or move | Selects an existing workspace, creates the next unused positive numbered workspace, or moves the selected client. |
| Remove empty last workspace | Leaves the currently selected final workspace if it is empty and another workspace exists on that output. Sway retires the empty workspace. |

Sway has no per-window always-on-top request, so the overview disables that action. On another compositor the standalone windowed preview can use its ordinary Wayland surface. Sway-specific desktop controls require a matching compositor IPC endpoint.

## Native lifecycle test

Use a private display to test workspace actions. The ignored test launches an independent `foot` client, observes its xdg-shell configure and surface commits, and checks discovery, focus, placement, fullscreen, workspace movement, minimize, restoration and close. It requires Sway, swaymsg and foot.

From the repository root, run the build first, then start the test as an ordinary user.

```sh
cargo test --locked -p ferro-desktop --no-run
FERRO_WAYLAND_PRIVATE_TEST=1 scripts/wayland/sway-run.sh --size 1365x768 -- \
  cargo test --locked -p ferro-desktop sway_native_client_lifecycle -- --ignored --test-threads=1
```

Protocol details are available in [Sway IPC](https://github.com/swaywm/sway/blob/master/sway/sway-ipc.7.scd).
