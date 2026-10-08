# NOTICE

FerroUI is a Rust port of the Avalonia UI framework. Its architecture, public API shape, algorithms and a large part of its behaviour are derived from the Avalonia sources, and some components contain directly imported upstream code. This file records the upstream projects and their licenses. Component-level notices with file mappings live next to the code they cover.

## Avalonia

- Project: <https://github.com/AvaloniaUI/Avalonia>
- License: MIT
- Used for: the whole framework design and implementation that the crates under `src/` port (base library, controls, markup, rendering and platform backends), the unit tests ported alongside them, and the native macOS backend imported under `native/FerroUI.Native/` (see `native/FerroUI.Native/NOTICE.md`).

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

Avalonia itself incorporates code from other projects (for example WPF-derived layout and text code under the MIT license from the .NET Foundation, and Chromium-derived snippets in the native macOS backend); those notices are carried in the upstream `NOTICE.md` and, where the corresponding code is ported or imported here, in the component-level notices.

## XamlX

- Project: <https://github.com/kekekeks/XamlX>
- License: MIT, Copyright (c) 2019 Nikita Tsukanov
- Used for: the XAML compiler front end ported under `external/XamlX/` (see `external/XamlX/NOTICE.md`).

## MicroCom

- Project: <https://github.com/kekekeks/MicroCom>
- License: MIT
- Used for: the design of the COM-style interop runtime and IDL code generator under `src/FerroUI.MicroCom/` and `src/tools/MicroCom.CodeGenerator/` (see the `NOTICE.md` files there).

## .NET runtime

- Project: <https://github.com/dotnet/runtime>
- License: MIT, Copyright (c) .NET Foundation and Contributors
- Used for: the masked text engine `src/FerroUI.Controls/utils/masked_text_provider.rs`, derived from `System.ComponentModel.MaskedTextProvider` (the class the upstream masked text box is built on).

## Code under other licenses

Some ported files derive from sources that the upstream project took from third parties under licenses other than MIT. They keep their original header lines, and the component-level notice lists them with the full license text:

- Silverlight Toolkit, Microsoft Public License (Ms-PL): the `AutoCompleteBox` sources and the selection adapters under `src/FerroUI.Controls/` (see `src/FerroUI.Controls/NOTICE.md`). Ms-PL requires that source distributions of those portions stay under Ms-PL.
- WPF and WinUI (MIT, .NET Foundation / Microsoft Corporation): see `src/FerroUI.Controls/NOTICE.md`, `src/FerroUI.Controls.ColorPicker/NOTICE.md` and the file headers in `src/FerroUI.Base/media/`.
- Inter typeface (SIL Open Font License 1.1): the font files embedded by `src/FerroUI.Fonts.Inter` (see `src/FerroUI.Fonts.Inter/NOTICE.md`).
- Roboto typeface (Apache License 2.0, Copyright 2011 Google Inc.): the font file of the about dialog embedded by `src/FerroUI.Dialogs` (see `src/FerroUI.Dialogs/NOTICE.md`).
- Test fonts of `ferroui-base` (SIL Open Font License 1.1: Inter, Adobe Blank 2; Apache License 2.0: two files derived from WenQuanYi Micro Hei): used by its tests only (see `src/FerroUI.Base/NOTICE.md`).
- wayland-protocols (MIT-style): popup positioner documentation and flag names, see `src/FerroUI.Controls/NOTICE.md`.

## Third-party crates

Rust dependencies are used under their own licenses as declared in their crate metadata; they are not vendored in this repository. Direct dependencies: `skia-safe` (MIT), `harfbuzz-sys` (MIT), `bitflags`, `paste`, `cc`, `roxmltree`, `time` (MIT OR Apache-2.0), `rust_decimal` (MIT) and `sysinfo` (MIT).
