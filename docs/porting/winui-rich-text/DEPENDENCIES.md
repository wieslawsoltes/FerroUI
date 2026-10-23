# Rich text source and dependency boundaries

Generated alongside the inventory. Source links are pinned to the same commit as the implementation map.

## Available source and external engines

| Dependency | Evidence at the pinned revision | Disposition |
|---|---|---|
| Editable RichEdit engine | [`dxaml/xcp/core/native/text/Controls/TextBoxBase.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/native/text/Controls/TextBoxBase.cpp), `CTextBoxBase::InitializeTextServices`, loads `WinUIEdit.dll` and obtains `CreateTextServices`/`ShutdownTextServices`. | The public repository supplies the control and host; its engine implementation is external. Implement the observable editing contracts in Rust. |
| Rich text object model metadata | [`eng/productmetadata.props`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/eng/productmetadata.props) names `Microsoft.UI.Text.winmd` from `WinUIDetailsLibPath` with `ImplementationDll` `WinUIEdit.dll`. [`packaging/Intellisense/generated/Microsoft.UI.Text.xml`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/packaging/Intellisense/generated/Microsoft.UI.Text.xml) documents its public members. | Metadata/docs can define a semantic target; they do not provide implementation source. All documented members are retained in `api-members.json`. |
| Internal dependency package | [`eng/Versions.props`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/eng/Versions.props) selects `Microsoft.Internal.WinUIDetails`; [`eng/winuidetails.targets`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/eng/winuidetails.targets) imports its targets. | Do not label externally delivered source/headers as present in this repository or redistributable under the repository's MIT license. |
| LineServices | [`dxaml/xcp/core/text/RichTextServices/inci/Ls.h`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextServices/inci/Ls.h) includes `Ptls-min.h`; [`dxaml/xcp/core/text/RichTextServices/TextFormatter/LsTextFormatter.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextServices/TextFormatter/LsTextFormatter.cpp) constructs the callback table and calls the external line engine. | The header and engine implementation are absent from the tracked tree. Reuse FerroUI formatting and audit the semantic role of every visible host callback. |
| DirectWrite | [`dxaml/xcp/pal/win/DWriteFontAndScriptServices.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/pal/win/DWriteFontAndScriptServices.cpp), `DWriteFontFace`, `DWriteTextAnalyzer` and their PAL contracts wrap platform APIs. | Existing FerroUI font/shaper/backend APIs own platform integration; no second font/rasterization subsystem is introduced by the rich control. |
| Direct2D, WIC and text bitmap composition | [`dxaml/xcp/core/native/text/Common/D2DTextDrawingContext.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/native/text/Common/D2DTextDrawingContext.cpp) and native `HWRenderTarget`/brush/bitmap classes. | Map rendering semantics onto `DrawingContext`, existing glyph runs, brushes, clipping and render invalidation. Backend rasterization is separate from document behavior. |
| Text input, linguistics and windowless host | [`dxaml/xcp/core/native/text/Controls/TextServicesHost.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/native/text/Controls/TextServicesHost.cpp), `LinguisticsHelper`, `PrivateTextInputSettings`, `TextBoxUIManagerEventSink`, `WindowLessSiteHost`. | Reuse FerroUI input/IME/clipboard/dispatcher contracts. Proofing, candidate, handwriting and Windows UI-manager features remain explicit service boundaries. |
| Accessibility | Native rich-edit and rich-text automation peers plus [`dxaml/xcp/core/inc/TextProvider.h`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/inc/TextProvider.h) / [`dxaml/xcp/core/inc/TextRangeProvider.h`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/inc/TextRangeProvider.h). | Existing automation peers are integration points, but full text/range patterns and embedded-object behavior require implementations and platform verification. |

## Include graph

The machine-readable graph records **976 additional framework files** reached from the selected scope and **846 boundary edges**. It reads source-relative paths first and otherwise follows only unique repository path suffixes. Windows case-insensitive lookup is applied to source paths.

This is a source-dependency audit, not a compiler dependency dump. Conditional compilation, build include-path precedence, SDK/private-package headers, generated headers, runtime interface acquisition and linker inputs cannot be inferred from filename matching. Ambiguous and absent includes are preserved with their referring file and line; traversal stops at those boundaries. `external_or_generated` means absent from this Git tree, not proof that an API is proprietary or unavailable elsewhere.

General dependency-object, rendering, input, UI Automation and DXAML framework headers appear in the transitive graph because the text host uses the framework. They are not automatically added as independent source ports: their behavior must map to an existing FerroUI API or receive a separately reviewed implementation. A matching basename is insufficient evidence of such a mapping.

## Frequent boundary includes

| Include | Resolution | Referring edges |
|---|---|---:|
| `precomp.h` | ambiguous | 144 |
| `vector` | external_or_generated | 29 |
| `XamlTailored.h` | ambiguous | 26 |
| `pch.h` | ambiguous | 26 |
| `NamespaceAliases.h` | ambiguous | 22 |
| `functional` | external_or_generated | 19 |
| `ppltasks.h` | external_or_generated | 16 |
| `cstdint` | external_or_generated | 15 |
| `type_traits` | external_or_generated | 15 |
| `utility` | external_or_generated | 14 |
| `memory` | external_or_generated | 13 |
| `array` | external_or_generated | 10 |
| `microsoft.ui.composition.h` | external_or_generated | 10 |
| `windows.h` | external_or_generated | 10 |
| `macros.h` | ambiguous | 9 |
| `unordered_set` | external_or_generated | 9 |
| `map` | external_or_generated | 8 |
| `math.h` | external_or_generated | 8 |
| `unordered_map` | external_or_generated | 8 |
| `DoubleUtil.h` | ambiguous | 7 |
| `StaticAssertFalse.h` | ambiguous | 7 |
| `UIAutomationCore.h` | external_or_generated | 7 |
| `optional` | external_or_generated | 7 |
| `string` | external_or_generated | 7 |
| `algorithm` | external_or_generated | 6 |
| `chrono` | external_or_generated | 6 |
| `windows.foundation.collections.h` | external_or_generated | 6 |
| `Microsoft.UI.Content.h` | external_or_generated | 5 |
| `Microsoft.UI.Xaml.hosting.referencetracker.h` | external_or_generated | 5 |
| `microsoft.ui.composition.experimental.h` | external_or_generated | 5 |
| `stdlib.h` | external_or_generated | 5 |
| `textserv.h` | external_or_generated | 5 |
| `wil\resource.h` | external_or_generated | 5 |
| `DWrite_3.h` | external_or_generated | 4 |
| `SimpleProperties.h` | ambiguous | 4 |
| `TextInput.h` | external_or_generated | 4 |
| `UIAutomation.h` | external_or_generated | 4 |
| `XamlOM.WinUI.h` | external_or_generated | 4 |
| `cstddef` | external_or_generated | 4 |
| `float.h` | external_or_generated | 4 |
| `intrin.h` | external_or_generated | 4 |
| `iterator` | external_or_generated | 4 |
| `limits` | external_or_generated | 4 |
| `mutex` | external_or_generated | 4 |
| `shlwapi.h` | external_or_generated | 4 |
| `D2D1.h` | external_or_generated | 3 |
| `InputScope.h` | external_or_generated | 3 |
| `Microsoft.UI.Xaml.private.h` | external_or_generated | 3 |
| `TextAnalysis.h` | external_or_generated | 3 |
| `TraceLoggingProvider.h` | external_or_generated | 3 |
| `WexTestClass.h` | external_or_generated | 3 |
| `d2d1_1.h` | external_or_generated | 3 |
| `flags_enum.h` | ambiguous | 3 |
| `forward_list` | external_or_generated | 3 |
| `framework.h` | ambiguous | 3 |
| `gsl/span` | external_or_generated | 3 |
| `hstring.h` | external_or_generated | 3 |
| `inspectable.h` | external_or_generated | 3 |
| `intsafe.h` | external_or_generated | 3 |
| `list` | external_or_generated | 3 |
| `malloc.h` | external_or_generated | 3 |
| `microsoft.ui.input.h` | external_or_generated | 3 |
| `new.h` | external_or_generated | 3 |
| `objbase.h` | external_or_generated | 3 |
| `ole2.h` | external_or_generated | 3 |
| `shobjidl_core.h` | external_or_generated | 3 |
| `stack` | external_or_generated | 3 |
| `stdint.h` | external_or_generated | 3 |
| `strsafe.h` | external_or_generated | 3 |
| `tchar.h` | external_or_generated | 3 |
| `vadefs.h` | external_or_generated | 3 |
| `wil\common.h` | external_or_generated | 3 |
| `windows.foundation.numerics.h` | external_or_generated | 3 |
| `D2D1_1.h` | external_or_generated | 2 |
| `D2d1.h` | external_or_generated | 2 |
| `D3d11.h` | external_or_generated | 2 |
| `FrameworkUdk/BackButtonIntegration.h` | external_or_generated | 2 |
| `Microsoft.DirectManipulation.h` | external_or_generated | 2 |
| `Microsoft.UI.Xaml.coretypes.h` | external_or_generated | 2 |
| `MuiUdk.h` | external_or_generated | 2 |

The full boundary list and all candidate matches are retained in `inventory.json`; this summary is intentionally bounded.
