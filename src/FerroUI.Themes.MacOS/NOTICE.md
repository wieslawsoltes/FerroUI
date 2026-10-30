# NOTICE

This macOS 26 theme is derived from FerroUI’s WinUI theme and retains its
Microsoft and Avalonia attribution. The macOS templates and semantic resources
are independently authored adaptations based on public Apple design guidance.
No Apple framework implementation, system fonts or SF Symbols assets are bundled.
See `docs/porting/macos26-theme.md` for provenance and measured fidelity scope.

The markup documents of this crate are ported from the WinUI 3 theme of
microsoft-ui-xaml (<https://github.com/microsoft/microsoft-ui-xaml>, branch
`winui3/main`, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`):
`dxaml/xcp/dxaml/themes/generic.xaml`, the `*_themeresources.xaml` documents and
the default style documents of `controls/dev`, and the density documents of
`controls/dev/dll/DensityStyles`. Each ported document names its source documents
in its header comment; `docs/porting/winui-theme.md` at the root of the repository
describes the port. The original resource documents were converted by
`scripts/winui-theme/convert.py`; this copied package maintains its macOS resources
and control templates independently. `Converters/corner_radius_to_thickness_converter.rs` is a Rust port
of the class `CornerRadiusToThicknessConverter` of the same repository
(`controls/dev/Common/CornerRadiusToThicknessConverter.idl`, `.h` and `.cpp`), and
`Primitives/info_bar_panel.rs` a Rust port of the class `InfoBarPanel`
(`controls/dev/InfoBar/InfoBarPanel.cpp`, `.h` and `InfoBar.idl`), and
`Primitives/tab_view_equal_width_panel.rs` a Rust port of the sizing of the tabs of
`TabView` in its `Equal` width mode (`TabView::UpdateTabWidths` of
`controls/dev/TabView/TabView.cpp`, `.h` and `TabView.idl`).

## microsoft-ui-xaml

The ported documents are subject to the following license:

```
MIT License

Copyright (c) Microsoft Corporation. All rights reserved.

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
SOFTWARE
```

## Avalonia

The Rust code of the crate (the theme class, the accent colours, the build script
and the tests) and the templates of the controls that WinUI 3 has no style for
(the window, popup and adorner hosts) are patterned on the Fluent theme of
FerroUI (`src/FerroUI.Themes.Fluent`), which is converted from the Fluent theme of
the upstream project, Avalonia (MIT; see `NOTICE-PROJECT.md`, the `NOTICE.md` at
the root of the repository). The outlines `GlyphGeometryE73E` and
`GlyphGeometryE894` of `Resources/Glyphs.xaml` are the check mark and the clear
button outlines of that theme.

## Symbol font

The templates of microsoft-ui-xaml show some glyphs of the Segoe Fluent Icons
font of Windows. The font is not part of this crate and is not redistributed:
`Resources/Glyphs.xaml` draws each glyph the port needs as a geometry.
