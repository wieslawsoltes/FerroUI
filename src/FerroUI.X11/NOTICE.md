# NOTICE

This crate is ported from the `Avalonia.X11` project of Avalonia (MIT; see the
`NOTICE.md` at the root of the repository), and `raw_event_grouping.rs` from
the shared source file `src/Shared/RawEventGrouping.cs` of the same project.

## Files with a licence header of their own

Two files of the upstream project carry the header of the project they were
taken from (the Windows Forms implementation of Mono), and their ports carry
it with them:

- `x11_structs.rs`, ported from `X11Structs.cs` (the enumerations and the
  Motif hints; the structures of that file are those of the Xlib bindings).
  Copyright (c) 2004 Novell, Inc. Authors: Peter Bartok (pbartok@novell.com).
- `x11_atoms.rs`, ported from `X11Atoms.cs`.
  Copyright (c) 2006 Novell, Inc. (https://www.novell.com)

Both under this licence:

```text
Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

## Tables transcribed from the X Window System

- `keysyms.rs` is the key symbol table of the upstream project
  (`Keysyms.cs`), which is a transcription of `X11/keysymdef.h` of the X.Org
  Foundation (names and values of the key symbols).
- The scan code table of `x11_key_transform.rs` follows, as its upstream file
  says, the table of the Chromium project
  (`ui/events/keycodes/dom/dom_code_data.inc`): key codes and the names of
  the physical keys.
- The cursor shapes of `x11_structs.rs` (`CursorFontShape`) are the values of
  `X11/cursorfont.h`.

## Dependencies

| Crate | Version | Licence | Used for |
|---|---|---|---|
| `x11-dl` | 2.21 | MIT (the bindings state they are public domain; the X11 libraries they describe are under the MIT licence) | The declarations of Xlib and of its extension libraries (Xi, Xrandr, Xcursor, Xfixes, Xext), which are opened at run time. Nothing of the X libraries is linked or distributed with the crate. |
| `libc` | 0.2 | MIT OR Apache-2.0 | The system calls of the event loop (`epoll`, `poll`, the pipe), `setlocale` and `gethostname`. |

`x11-dl` depends on `libc`, `once_cell` (MIT OR Apache-2.0) and, at build
time, `pkg-config` (MIT OR Apache-2.0).

The X libraries themselves (`libX11`, `libXi`, `libXrandr`, `libXcursor`,
`libXfixes`, `libXext`) are those of the system the application runs on, under
the MIT/X11 licences of the X.Org Foundation.
