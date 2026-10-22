# Requested-range input geometry

The platform input bridge can query the geometry of an explicit UTF-16 range in
the visible rich document, including composition preview, without changing its
selection, scroll position, composition or undo history. It uses the same shaped
lines and linked pages that draw the control.

Microsoft's [CoreTextLayoutRequest.Range](https://learn.microsoft.com/en-us/uwp/api/windows.ui.text.core.coretextlayoutrequest.range)
contract identifies the range requested by the text input server.
[CoreTextLayoutBounds.TextBounds](https://learn.microsoft.com/en-us/uwp/api/windows.ui.text.core.coretextlayoutbounds.textbounds)
requires its bounding rectangle in screen coordinates. A request for one range
therefore cannot generally be answered with the user's current caret rectangle.

## Shared client capability

[`TextInputMethodClient::text_range_geometry`](../../../src/FerroUI.Base/input/text_input/text_input_method_client.rs)
is an optional read-only capability. Its immutable
[`TextInputMethodRangeGeometry`](../../../src/FerroUI.Base/input/text_input/text_input_method_range_geometry.rs)
echoes the exact requested range and retains each shaped rectangle separately
from its transform to the semantic input owner. A client-supplied observational
guard uses weak references and is surrounded by checks of the framework's visual
geometry version.

| Query outcome | Adapter behavior |
|---|---|
| Complete current geometry for the echoed range | Transform its fragments and publish the resulting bounds. |
| Unsupported capability | Use the existing legacy caret compatibility path. |
| InvalidRange | Publish no geometry for the invalid request. |
| Unavailable | Publish no geometry for a missing, unmeasured, hidden, trimmed or partly covered range. |
| StateChanged, stale guard or mismatched echoed range | Discard the old result and publish no geometry. |

Only Unsupported authorizes the compatibility path. Zero-width fragments are
valid for structural separators; an empty fragment list means unavailable.
Negative dimensions, nonfinite rectangles and nonfinite transformed corners are
rejected. A geometry snapshot is observational and never authorizes an edit or
keeps an input session alive.

## Rich document coverage

[`rich_edit_input_range_geometry.rs`](../../../src/FerroUI.Controls/rich_edit_input_range_geometry.rs)
checks ordered endpoints against the current visible story. Nonempty ranges use
the actual formatter's shaped bounds. A query may address a position inside a
UTF-16 surrogate or shaped cluster because geometry does not perform an edit;
the resulting rectangle follows the shared formatter's contract. Editing keeps
its separate scalar-boundary validation.

An empty range describes the requested insertion position, after verifying that
a visible formatted line contains it. A real paragraph separator contributes a
zero-advance line-height box, rather than the wider painted selection marker.
Inline breaks, bidi fragments, embedded controls and linked overflow use the
same source coverage calculation. Every requested unit must be covered; omitted
trimmed content cannot silently become a nearby caret.

The query walks a coherent linked chain and records document mutation identity,
editor/selection/composition identity, view bounds, text origin, transforms, next
links, layout generation and cached layout identity. Changing any of that state
retires the result. Formatted content outside a scrolling viewport keeps its
translated bounds: this query is not an ancestor-clipping or UIA visibility test.

## Callback ownership and cache publication

Text source construction, custom runs, font services, transforms and native
property access can invoke application code. RichTextBlock therefore releases
its source-cache borrow before creating runs, captures a layout generation, and
publishes a source or layout only while that generation remains current. A newer
nested layout survives an older outer callback. Geometry validation checks
cached identity without performing a new layout as its final validation step.

The Windows adapter also checks request/session/client ownership around native
getters, transforms and setters. It transforms each fragment's four corners
through the semantic owner and presentation root before taking its bounding box,
then applies rasterization scale and HWND screen origin. This avoids repeatedly
flattening rotated rectangles at intermediate coordinate spaces. The complete
semantic control rectangle is transformed independently. Exact range bounds do
not receive the legacy minimum caret inflation.

## Source mapping and limits

At the pinned WinUI reference,
[`RichTextBlockView.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlockView.cpp)
provides text bounds, position-to-pixel and containment contracts.
[`LinkedRichTextBlockView.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/LinkedRichTextBlockView.cpp)
checks linked-view position containment, but its range-to-bounds entry returns
E_NOTIMPL. The coherent linked aggregation here is an input capability built from
FerroUI views, not a claim that that unimplemented native entry was translated.
[`TextServicesHost.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/native/text/Controls/TextServicesHost.cpp)
and [`TextBoxView.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/native/text/Controls/TextBoxView.cpp)
separately ground client/screen conversion and the control viewport.

The new tests cover the Base capability, real rich clients, portable CoreText
transforms and native Windows callback ownership. Runtime evidence is recorded
in [TESTS.md](TESTS.md). Formatter-defined cluster geometry, opaque native glyph
metrics, candidate-window UI, proofing/handwriting services, composition clause
formatting and real-device IME sessions remain separate conformance work.
