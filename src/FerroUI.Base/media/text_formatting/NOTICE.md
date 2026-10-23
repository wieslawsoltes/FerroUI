# Paragraph tab contract notices

The tab alignment and leader enumerations and the explicit tab-stop contract are
translated from the Microsoft WinUI public rich-text contract, pinned at
`fe244ca6ac4d482712b234a9de3cf771cc6481de`,
`packaging/Intellisense/generated/Microsoft.UI.Text.xml`.

Copyright (c) Microsoft Corporation. All rights reserved.

The Rust position-dependent layout is an independent integration with FerroUI's
existing shaping, bidi, glyph-run, line wrapping, drawing and property services.
The native external RichEdit and LineServices engines are not present in the
public reference tree. Native pixel and platform conformance remain separately
tracked in `docs/porting/winui-rich-text/`.

This notice applies to `text_tab_stop.rs`, `text_tab_layout.rs` and their tab
integration in the surrounding formatter. Existing formatter source attribution
is retained by the containing crate's notices.

## MIT License

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
