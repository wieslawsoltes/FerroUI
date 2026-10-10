# NOTICE

This crate is ported from the `Avalonia.Wayland` project of Avalonia (MIT; see
the `NOTICE.md` at the root of the repository). It uses three source files of
the `Avalonia.X11` project and one shared source file of the same repository
through their ports in the crate `ferroui-x11` (the icon loader, the dispatcher
over the main loop of GLib with its interop, and the grouping of raw input).

No file of the upstream project carries a licence header of its own.

## Dependencies

Pinned exactly in the manifest of the workspace; versions and licences as
`cargo info` reported them on 2026-10-10. Nothing of them is linked into an
application at build time: the C libraries they declare are opened at run time.

| Crate | Version | Licence | What it is here |
|---|---|---|---|
| `wayland-client` | 0.31.15 | MIT | The protocol objects, the event queue and the core protocol |
| `wayland-backend` | 0.3.17 | MIT | The wire, over `libwayland-client.so.0` (features `client_system`, `dlopen`) |
| `wayland-sys` | 0.31.11 | MIT | The C declarations of `libwayland-client` and `libwayland-egl`, opened with `dlopen` (a dependency of the two above and of `wayland-egl`) |
| `wayland-scanner` | 0.31.11 | MIT | The generator of the protocol bindings, at build time |
| `wayland-protocols` | 0.32.13 | MIT | The bindings of `xdg-shell`, `xdg-decoration`, `xdg-output` and the other protocol extensions |
| `wayland-cursor` | 0.31.14 | MIT | The cursor themes (with `xcursor` 0.3, MIT) |
| `wayland-egl` | 0.32.11 | MIT | `wl_egl_window` of `libwayland-egl.so.1` |
| `xkbcommon-dl` | 0.4.2 | MIT | The C declarations of `libxkbcommon.so.0`, opened with `dlopen` (with `xkeysym` 0.2, MIT OR Apache-2.0 OR Zlib, and `dlib` 0.5, MIT) |
| `wayland-protocols-wlr` | 0.3.12 | MIT | Only for the example: the virtual pointer and screen copy protocols of wlroots |
| `wayland-protocols-misc` | 0.3.12 | MIT | Only for the example: the virtual keyboard protocol |

What these bring that the workspace did not have: `dlib` 0.5 (MIT) with
`libloading` 0.8 (ISC), `downcast-rs` 1.2 and `scoped-tls` 1.0 (MIT OR
Apache-2.0), `xcursor` 0.3 (MIT), `xkeysym` 0.2, and at build time `quick-xml`
(MIT), which the generator reads the protocol descriptions with.

The protocol descriptions the bindings are generated from are part of those
crates and carry the licences of their protocols (MIT-style notices of the
Wayland project, of the wlroots project and of the authors named in each
file).

## System libraries opened at run time

`libwayland-client.so.0` and `libwayland-egl.so.1` (MIT), `libxkbcommon.so.0`
(MIT), `libEGL.so.1` (the EGL of the system: Mesa is MIT; a vendor driver has
its own terms), and through the X11 crate `libglib-2.0.so.0` when the option
`use_g_lib_main_loop` is set (LGPL-2.1-or-later). None of them is distributed
with this crate.
