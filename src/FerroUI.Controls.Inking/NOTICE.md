# WinUI inking attribution

The controls, service contracts, default palettes, localized resource tables,
preview sample points and matching behavioral tests
in this directory are derived from or informed by the Microsoft WinUI sources:

- Repository: https://github.com/microsoft/microsoft-ui-xaml
- Revision: `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`
- Source directories: `controls/dev/InkCanvas` and `controls/dev/InkToolbar`

The complete source and member mapping is recorded in
`docs/porting/data/winui-inking/` and explained in
`docs/porting/winui-inking.md`. Multiple small C++ declarations and IDL contracts
may share a Rust file as recorded there. Windows Runtime and DirectComposition
dependencies are replaced by FerroUI services or explicitly tracked as missing;
their operating-system implementation is not included in the WinUI repository
and has not been copied here.

The generated `ink_toolbar_strings_data.rs` retains all 86 source cultures and
59 resource keys per culture. `scripts/winui-inking/generate-strings.py` checks
each source Git blob before regenerating those tables. The dedicated toolbar
themes in `src/FerroUI.Controls.Inking/Themes/WinUI/InkToolbar.xaml` and
`src/FerroUI.Controls.Inking/Themes/WinUI/InkToolbarResources.xaml` are adaptations of the same
pinned WinUI XAML and use the Microsoft notice below. Their tool icons use
FerroUI vector geometry; this port does not redistribute the Windows symbol
font.

## Microsoft WinUI license

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
SOFTWARE.

## Optional Windows interoperability dependencies

`src/FerroUI.Controls.Inking/windows_ink_services.rs` calls the installed Windows ink APIs through
the Microsoft Rust for Windows projections: `windows` 0.62.2,
`windows-collections` 0.3.2 and `windows-numerics` 0.3.1. These dependencies are
licensed under MIT OR Apache-2.0 and retain their license files in the published
crates. Their repository is <https://github.com/microsoft/windows-rs>. They are
target-specific Cargo dependencies, not copied sources in this directory.
The declaration-only snapshot
`docs/porting/data/winui-inking/upstream/windows-0.62.2-inking-services.txt`
is derived from `microsoft/windows-rs` revision
`32c3144490c016fe496a0aed769bce60987a2e9d`, Copyright (c) Microsoft Corporation,
under the MIT option of that project's MIT OR Apache-2.0 license. The Microsoft
MIT permission and warranty text is retained above; no projection method bodies
or operating-system implementation are included in that snapshot.
The Windows ink engine, native recognizers and language data are OS services;
this port does not distribute their implementation or data.
