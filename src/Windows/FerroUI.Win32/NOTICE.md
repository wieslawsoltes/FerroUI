# Third-party notices

## Avalonia

The Windows platform backend is a port of `src/Windows/Avalonia.Win32` of the
Avalonia project (MIT, Copyright (c) AvaloniaUI OÜ; the license text is in the
`NOTICE.md` at the root of the repository): its structure, its names, its
logic and the comments that explain that logic are derived from it, file by
file. The mapping of the files is in `docs/porting/win32-platform.md`.

The upstream sources carry code and comments adapted from other projects,
which this port carries with them:

| Where | From | License |
|---|---|---|
| `input/key_interop.rs` (the virtual key of the generic modifier keys), `window_impl.rs` (the clean-up before a window closes), `win32_dispatcher_impl.rs` (the pending input check and its comment) | WPF (`HwndKeyboardInputProvider.cs`, `Window.cs`, the dispatcher), https://github.com/dotnet/wpf | MIT, Copyright (c) .NET Foundation and Contributors |
| `window_impl.rs` (`set_full_screen`) | Chromium (`ui/views/win/fullscreen_handler.cc`) | BSD 3-Clause, Copyright The Chromium Authors |
| `input/key_interop.rs` (the table of scan codes) | compiled from Chromium's `ui/events/keycodes/dom/dom_code_data.inc` and Microsoft's documentation of scan codes | BSD 3-Clause, Copyright The Chromium Authors |

```
Copyright (c) .NET Foundation and Contributors

All rights reserved.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

```
Copyright 2015 The Chromium Authors

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

   * Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.
   * Redistributions in binary form must reproduce the above
copyright notice, this list of conditions and the following disclaimer
in the documentation and/or other materials provided with the
distribution.
   * Neither the name of Google LLC nor the names of its
contributors may be used to endorse or promote products derived from
this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

## Crates the backend links

On Windows the backend links the crates below from crates.io. Both are
licensed under the terms of both the Apache License, Version 2.0 and the MIT
license, at the choice of the user; FerroUI uses them under the MIT license.
The texts of the licenses ship with each crate (`license-apache-2.0`,
`license-mit`).

| Crate | Version | Used for | Copyright |
|---|---|---|---|
| `windows-sys` | 0.61.2 | the declarations of the Windows API functions, structures and constants the backend calls | Copyright (c) Microsoft Corporation |
| `windows-link` | 0.2.1 | linking those functions without import libraries (a dependency of `windows-sys`) | Copyright (c) Microsoft Corporation |

### ANGLE (the feature `angle`, on by default)

With its feature `angle` the backend links ANGLE, an implementation of
OpenGL ES and EGL on Direct3D, through the crate `mozangle`, whose build
script compiles ANGLE from C++ source and links it statically into the
application: the application carries ANGLE and has to carry its notice.

| Crate | Version | Used for | License | Copyright |
|---|---|---|---|---|
| `mozangle` | 0.7.1 | ANGLE as Mozilla ships it in Firefox (`gfx/angle` of `FIREFOX_153_3_0esr_RELEASE`), packaged as a crate by the Servo project: the EGL and OpenGL ES entry points of the rendering mode `AngleEgl` | BSD-3-Clause | The ANGLE Project Authors; The Servo Project Developers |
| `libz-sys` | 1.1.29 | zlib, which the sources of ANGLE use (compiled from the source the crate carries when the system has none) | MIT OR Apache-2.0 (the crate); the zlib license (zlib itself) | Alex Crichton, Josh Triplett, Sebastian Thiel; Jean-loup Gailly and Mark Adler |

The sources of ANGLE the crate compiles include code of other projects
under licenses of the same kind (parts of Chromium's `base`, SystemInfo,
MurmurHash by Austin Appleby, xxHash by Yann Collet); their notices are in
the directories of those sources in the crate (under `gfx/angle/checkout/src`). The crates the build script of
`mozangle` runs with (`bindgen`, `cc`, `gl_generator`, `khronos_api`,
`walkdir`) are tools of the build and are not linked.

The license of ANGLE:

```
Copyright (C) 2002-2013 The ANGLE Project Authors.
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:

    Redistributions of source code must retain the above copyright
    notice, this list of conditions and the following disclaimer.

    Redistributions in binary form must reproduce the above
    copyright notice, this list of conditions and the following
    disclaimer in the documentation and/or other materials provided
    with the distribution.

    Neither the name of TransGaming Inc., Google Inc., 3DLabs Inc.
    Ltd., nor the names of their contributors may be used to endorse
    or promote products derived from this software without specific
    prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.
```

The example of the crate (`examples/win32_window.rs`) draws with the Vello
backend of FerroUI, whose crates are listed in
`src/Vello/FerroUI.Vello/NOTICE.md`; they are not linked into the backend.
