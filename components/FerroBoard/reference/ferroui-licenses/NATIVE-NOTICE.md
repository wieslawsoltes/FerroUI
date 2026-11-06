# NOTICE

The sources in this directory (`inc/`, `src/OSX/`) and the interface
definition file `src/FerroUI.Native/frn.idl` are derived from the native macOS
backend of the **Avalonia** project (<https://github.com/AvaloniaUI/Avalonia>):

| Here                                   | Upstream                               |
|----------------------------------------|----------------------------------------|
| `native/FerroUI.Native/inc/*.h`        | `native/Avalonia.Native/inc/*.h`       |
| `native/FerroUI.Native/src/OSX/*`      | `native/Avalonia.Native/src/OSX/*`     |
| `src/FerroUI.Native/frn.idl`           | `src/Avalonia.Native/avn.idl`          |

The upstream revision they were imported from is recorded in
`UPSTREAM_REVISION`. The import is done by `scripts/sync-native.sh`, which
copies the files and mechanically renames identifiers and file names
(`Avalonia.Native` -> `FerroUI.Native`, `Avalonia` -> `Ferro`, `Avn` -> `Frn`,
in all three casings); the code is otherwise unmodified.

The rename also removes the per-file copyright comment lines of the upstream
sources. They are reproduced here:

    // Copyright (c) The Avalonia Project. All rights reserved.
    // Licensed under the MIT license. See licence.md file in the project root for full license information.
        (inc/comimpl.h)
    // Copyright (c) 2022 Avalonia. All rights reserved.
    // Copyright © 2018 Avalonia. All rights reserved.
    // Copyright © 2019 Avalonia. All rights reserved.
    // Copyright © 2021 Avalonia. All rights reserved.
    // Copyright © 2022 Avalonia. All rights reserved.
    // Copyright © 2024 Avalonia. All rights reserved.
        (various files under src/OSX)

`src/OSX/main.mm` additionally contains a snippet marked
"Copyright (c) 2011 The Chromium Authors. All rights reserved."; that notice is
kept in place in the file.

## Upstream license

Avalonia is distributed under the MIT license, reproduced in full below
(from `licence.md` in the upstream repository):

```
The MIT License (MIT)

Copyright (c) AvaloniaUI OÜ
All Rights Reserved

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
