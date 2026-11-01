# Ferro Desktop

Ferro Desktop is a Linux desktop session built from FerroUI controls. The shell owns the desktop surface, application launcher and workspace overview. An EWMH window manager owns placement, stacking and focus of independent X11 clients. This boundary lets existing Linux applications participate without rendering their content a second time in the shell.

## Architecture

| Layer | Responsibility |
| --- | --- |
| Shell | Mount the visual tree once. Dispatch actions from buttons, search and keyboard input. |
| Applications | Discover the signed-in user's desktop entries and launch through Linux application services. |
| Window management | Observe the X server and delegate operations on real client windows to the window manager. |
| Presentation | Use FerroUI layout, text shaping, input, accessibility and the Skia compositor. Keep native graphics selection in the host. |

Windowed mode runs inside an existing X11 desktop. Login-session mode installs a desktop surface below application windows and uses the session window manager. The two modes share the same shell and application catalog.
