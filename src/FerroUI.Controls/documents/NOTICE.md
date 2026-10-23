# Text document implementation notices

The block, paragraph, block collection, hyperlink, typography and text-position contracts
are derived from Microsoft WinUI, pinned at commit
`fe244ca6ac4d482712b234a9de3cf771cc6481de`:

- `packaging/Intellisense/generated/Microsoft.UI.Text.xml` (paragraph tab APIs)
- `dxaml/xcp/core/inc/BlockTextElement.h`
- `dxaml/xcp/core/text/common/BlockCollection.cpp`
- `dxaml/xcp/core/text/RichTextArea/paragraph.cpp`
- `dxaml/xcp/core/text/RichTextArea/TextElement.cpp`
- `dxaml/xcp/core/text/common/text.cpp`
- `dxaml/xcp/core/text/common/TextFormatting.cpp`
- `dxaml/xcp/core/text/common/Typography.cpp`
- `dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Documents.cs`
- `dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.cs`
- `dxaml/xcp/core/text/RichTextServices/xcp/PALFontAndScriptServices.cpp`
- `dxaml/xcp/core/text/TextBlock/Hyperlink.cpp`
- `dxaml/xcp/core/inc/Hyperlink.h`
- `dxaml/xcp/core/text/Inc/TextPosition.h`
- `dxaml/xcp/core/text/common/TextPosition.cpp`
- `dxaml/xcp/core/text/Inc/PlainTextPosition.h`
- `dxaml/xcp/components/primitiveDependencyObjects/TextRange.cpp`
- `dxaml/xcp/components/primitiveDependencyObjects/inc/TextRange.h`
- `dxaml/xcp/components/text/TextRangeCollection.cpp`
- `dxaml/xcp/components/text/TextHighlighter.cpp`
- `dxaml/xcp/components/text/TextHighlighterCollection.cpp`
- `dxaml/xcp/components/text/TextHighlightMerge.cpp`
- `dxaml/xcp/components/text/TextHighlightRenderer.cpp`
- `dxaml/xcp/components/text/inc/HighlightRegion.h`
- `dxaml/xcp/components/text/unittests/TextHighlightMergeUnitTests.cpp`

Copyright (c) Microsoft Corporation. All rights reserved.

The shared Rust document, snapshot, format-patch, transaction and ownership
representations integrate these contracts with the existing FerroUI property,
inline, text-formatting and Unicode services. Structural element markers are
not exposed as character positions; the public offset coordinate system is
the UTF-16 text coordinate system used by the FerroUI formatter. The explicit
source/member inventory and the platform service boundary report live in
`docs/porting/winui-rich-text/`.

Text highlight ranges retain the signed start/length value contract while the
merge algorithm uses half-open UTF-16 endpoints. Markup initializes a shared box
and the observable range collection copies its values. Highlighter collections
retain their items; item, range and mutable-brush subscriptions retain owners
weakly. These layers affect rendering without changing document revisions or
editing history. The merge tests retain all ten named source overlap fixtures.

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
