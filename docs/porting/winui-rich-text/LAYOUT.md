# Rich line allocation and bounds

Rich layout uses the existing `TextFormatter`, `TextLine`, font selection,
shaper, drawing sink and visual layout APIs. This record separates the open
WinUI block-layout formulas from the native font and LineServices policies that
are still being verified. The source reference is
[`fe244ca6ac4d482712b234a9de3cf771cc6481de`](https://github.com/microsoft/microsoft-ui-xaml/tree/fe244ca6ac4d482712b234a9de3cf771cc6481de).

## Allocation and drawing

[`RichTextLayout`](../../../src/FerroUI.Controls/rich_text_layout.rs) keeps three
different values for each formatted line: its allocated advance, the top of that
allocation, and the origin at which the natural glyph box is drawn. The
formatter's natural height is not rewritten to implement a block's line height.
This distinction matters when a small line height makes ink extend outside its
allocation, and when the first or last line of a paragraph fragment trims space.

For natural line height `N`, requested line height `H`, and the regular paragraph
font family's baseline/advance ratio `r`, the pinned
[`ParagraphNode::ApplyLineStackingStrategy`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/BlockLayout/ParagraphNode.cpp)
supplies these formulas:

| Strategy | Allocated advance | Glyph-box offset from allocation top |
|---|---|---|
| MaxHeight | `max(N, H)` | `0` |
| BlockLineHeight | `H` | `(H - N) * r` |
| BaselineToBaseline, first fragment line | `H - (H - N) * r` | `0` |
| BaselineToBaseline, last fragment line | `H - (H - N) * (1 - r)` | `(H - N) * r` |
| BaselineToBaseline, interior line | `H` | `(H - N) * r` |

The first-line branch takes precedence when one line is both first and last.
A continued paragraph starts a new fragment on its overflow page. A page-height
or MaxLines boundary does not turn an unfinished paragraph into its last line.
An automatic/nonpositive line height uses natural allocation. FerroUI's NaN and
unbounded automatic-height sentinels also follow that natural-height path.

Page fitting advances by allocated space. Drawing and embedded-control baselines
use the glyph origin. Public line metrics, caret rectangles and ordinary block
range rectangles use allocation coordinates. Explicit rich-edit
`LineSpacingRule` values retain their separately implemented precedence and
formatter range-height policy; they are not silently converted to a block
stacking strategy.

## Reusing TextLineBounds

FerroUI main PR #294, merged at
[`0ce4e90c`](https://github.com/wieslawsoltes/FerroUI/commit/0ce4e90c20d89a03590ccdde555d5daa87e88ee6),
supplies `media::TextLineBounds`, cap-height metrics, paragraph-property forwarding
and the shared formatter implementation. RichTextBlock adds an owner to the
existing TextBlock property and uses the same enum, setter and markup system.

| Value | Numeric value | Requested box |
|---|---:|---|
| Full | 0 | Full ascent/descent allocation |
| TrimToCapHeight | 1 | Top starts at the cap height |
| TrimToBaseline | 2 | Bottom stops at the baseline |
| Tight | 3 | Cap height to baseline |

`RichTextSource` captures the document owner's mode. Every paragraph, empty
document and linked overflow page receives that mode through
`GenericTextParagraphProperties::set_text_line_bounds`. Changing it retires the
source and continuation caches without editing document content or history.
The source owner also controls overflow clipping, including pages in another
logical parent, relinking and detaching. Trimmed modes suppress ClipToBounds
through ordinary property coercion; returning to Full restores the user's base
choice.

The stacking ratio uses the native regular-family transform from
[`CCompositeFontFamily::GetTextLineBoundsMetrics`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/Fonts/CompositeFontFamily.cpp).
The natural line box in this checkpoint still comes from the existing shared
formatter. These two sources are deliberately identified independently.

## Native metric boundaries

The enum contract is exact. The rich property and complete native layout remain
partial for the following reasons:

| Contract | Current framework behavior and remaining boundary |
|---|---|
| Line gap | Existing Full metrics place half the gap above the baseline. Native family metrics keep the family baseline independent of that half gap. |
| Fallback glyphs | The formatter aggregates actual fallback glyph-face metrics. Native `LineServicesGetRunTextMetrics` uses the originally requested composite family. |
| Cap height | Main uses positive OS/2 sCapHeight, then a TrueType H outline, then a documented proportional fallback. DirectWrite's internal missing-cap-height policy is not in the public source. |
| Embedded objects | Main applies bounds to aggregate line metrics. The native formatter supplies an object's full height/baseline separately from bounded text metrics. |
| Property inheritance | The existing FerroUI attached property inherits. Native controls have individual, non-inherited TextLineBounds properties. |
| Engine output | The open block formulas do not reproduce opaque LineServices, native font rounding, shaping or rasterization. |

For equal supplied ascent `16`, descent `4`, line gap `2` and cap height `11.2`,
the source formulas illustrate the gap difference:

| Mode | Native family baseline / height | Current shared formatter baseline / height |
|---|---|---|
| Full | `16 / 22` | `17 / 22` |
| TrimToCapHeight | `11.2 / 17.2` | `11.2 / 16.2` |
| TrimToBaseline | `16 / 16` | `17 / 17` |
| Tight | `11.2 / 11.2` | `11.2 / 11.2` |

This is formula analysis with equal input metrics, not a measurement that
DirectWrite chooses those metrics for the same font. The underlying references
are `CompositeFontFamily.cpp`,
[`LineServicesGetRunTextMetrics`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextServices/TextFormatter/lineservicescallbacks.cpp),
[`LsTextLine::CalculateLineMetrics`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextServices/TextFormatter/LsTextLine.cpp),
and [`InlineObjectHandlers.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextServices/TextFormatter/InlineObjectHandlers.cpp).

## Reproducible verification

[`check-rich-text-line-stacking.py`](../../../scripts/porting/check-rich-text-line-stacking.py)
extracts and compiles the actual pinned C++ function and the current Rust helper.
Its harness supplies only primitive dependencies and a line-height getter.
It compares 960 combinations of strategy, family ratio, line height, natural
height and first/last flags, using relative tolerance `1e-6` or absolute `1e-5`
for native float versus Rust f64. The recorded result and function hashes are in
[`line-stacking-formula.json`](line-stacking-formula.json).

```sh
python3 scripts/porting/check-rich-text-line-stacking.py \
  --reference /path/to/microsoft-ui-xaml \
  --output docs/porting/winui-rich-text/line-stacking-formula.json
```

The independent real-control regressions are in
[`rich_text_line_stacking_tests.rs`](../../../src/FerroUI.Controls/rich_text_line_stacking_tests.rs)
and [`rich_text_line_bounds_tests.rs`](../../../src/FerroUI.Controls/rich_text_line_bounds_tests.rs).
They cover allocation, paragraph fragments, page fitting, inherited/local modes,
empty lines, fallback metrics, embedded controls, clipping, drawing, caret/range
bounds and explicit rich-edit spacing. The existing `rich_text_document.xaml`
corpus fixture exercises all four modes through runtime and emitted markup.
Their actual gate results belong in [TESTS.md](TESTS.md); the arithmetic oracle
alone is not a control or native-pixel pass.
