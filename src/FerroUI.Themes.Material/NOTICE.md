# Android Material 3 Expressive theme notices

## AndroidX / Material Design

Material numeric tokens, component geometry, state definitions, motion constants,
ripple timing and progress algorithms are adapted from AndroidX Compose Material3
1.5.0-beta02, commit `e4d933c98df7baa7ea2b83eea6e086915961e0ff`:
<https://android.googlesource.com/platform/frameworks/support/+/e4d933c98df7baa7ea2b83eea6e086915961e0ff/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/>.

Copyright The Android Open Source Project. Licensed under Apache License 2.0;
see `LICENSE-ANDROID`. These definitions have been translated and adapted for
FerroUI's Rust controls and XAML resource system. This is an independent port,
not a Google product or an Android platform implementation.

`material-colors` is used for CAM16/HCT dynamic palettes under MIT OR Apache-2.0.
Its version and algorithm baseline are recorded in the manifest and port guide.

The gallery share symbol uses Google Material Design Icons,
`src/social/share/materialicons/24px.svg` (blob `07fc6639ed2cf31fbd5e132aa0219d76afd68999`),
<https://github.com/google/material-design-icons>, Apache License 2.0.

## Google Material Color Utilities

`dynamic_color_spec.rs` ports the phone palette, color-role, contrast, and
tone-constraint rules from Google Material Color Utilities, commit
`bd811e4ca2ab94c4ce3cadc89c19b1820b9704b3`:
<https://github.com/material-foundation/material-color-utilities/tree/bd811e4ca2ab94c4ce3cadc89c19b1820b9704b3/typescript>.
Source files are `dynamiccolor/color_spec_2021.ts`, `color_spec_2025.ts`,
`dynamic_scheme.ts`, `dynamic_color.ts`, and `palettes/tonal_palette.ts`.
Copyright 2021, 2022, 2025 Google LLC. Licensed under Apache License 2.0; see
`LICENSE-ANDROID`. The reference fixtures execute the original TypeScript;
their source hashes and reproducible generation command are recorded in
`tests/fixtures/dynamic-color-provenance.json` and the dynamic-color port guide.

## Roboto

`Assets/Roboto-Regular.ttf`, `Assets/Roboto-Medium.ttf` and `Assets/Roboto-Bold.ttf`
are distributed under the SIL Open Font License 1.1, included as
`Assets/OFL-Roboto.txt`. They are static instances (weights 400, 500 and 700)
generated with fontTools from the unmodified Roboto variable font of
google/fonts, so font backends need not synthesize weights; the variable font
itself is not redistributed. Its revision, URL and SHA-256, and those of the
instances, are recorded in `scripts/material-theme/font-provenance.json`.

## WinUI / Avalonia structural heritage

The initial control-template contracts, resource aliases, build registration,
layout primitives and converters were copied from FerroUI's WinUI theme. The
following original notices remain applicable to that inherited material.


The markup documents of this crate are ported from the WinUI 3 theme of
microsoft-ui-xaml (<https://github.com/microsoft/microsoft-ui-xaml>, branch
`winui3/main`, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`):
`dxaml/xcp/dxaml/themes/generic.xaml`, the `*_themeresources.xaml` documents and
the default style documents of `controls/dev`, and the density documents of
`controls/dev/dll/DensityStyles`. Each ported document names its source documents
in its header comment; `docs/porting/winui-theme.md` at the root of the repository
describes the port. The resource documents under `Resources/` are converted by
`scripts/winui-theme/convert.py`; the control documents under `Controls/` are
ported by hand. `Converters/corner_radius_to_thickness_converter.rs` is a Rust port
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
the upstream project, Avalonia (MIT; see the project notice reproduced below and `NOTICE.md` at
the root of the repository). The outlines `GlyphGeometryE73E` and
`GlyphGeometryE894` of `Resources/Glyphs.xaml` are the check mark and the clear
button outlines of that theme.

## Symbol font

The templates of microsoft-ui-xaml show some glyphs of the Segoe Fluent Icons
font of Windows. The font is not part of this crate and is not redistributed:
`Resources/Glyphs.xaml` draws each glyph the port needs as a geometry.


---

Project-wide attributions and licence texts are included in `NOTICE-PROJECT.md`.
