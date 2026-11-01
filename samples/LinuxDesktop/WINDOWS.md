# Windows and workspaces

Open **Workspaces** with **Ctrl+W** in any appearance. Choose a workspace tab to switch, select a window title to activate its client, or use the buttons to minimize, maximize, restore or close it. The overview orders clients by recent activation. Unrelated rows keep their controls when a title or state changes.

The **Arrange or move** button opens actions for the selected window. Snap moves its frame to the left or right half of the monitor with the largest overlap. It uses the workspace's physical work area and subtracts the window manager's frame extents from the client size. Snapping out of maximize waits for the manager to restore decorations. The manager applies application constraints such as minimum size and resize increments. Fullscreen and always-on-top use the manager's actual state. A named workspace destination moves just that client.

**+** adds a workspace. **−** retires the last workspace when it is empty. The shell rechecks the server's workspace count and client assignments, including windows hidden from the taskbar, before sending the removal request.

The managed login session handles these shortcuts even while another application has focus.

| Shortcut | Action |
| --- | --- |
| Alt+Tab / Alt+Shift+Tab | Switch forward / backward through windows |
| Super+Left / Super+Right | Snap to the current monitor's left / right half |
| Super+Up / Super+Down | Toggle maximize / minimize |
| Super+1–4 / Super+Shift+1–4 | Switch workspace / send the focused client to that workspace |
| Alt+F4 | Ask the application to close its window |

Run the live integration checks with `samples/LinuxDesktop/tests/session.sh test`. The optional `samples/LinuxDesktop/tests/session.sh benchmark` runs 32 independent X11 clients and 200 title changes on a private display. It reports the shell connection's protocol request count and the time to dispatch a ready X11 event and publish the updated client list.
