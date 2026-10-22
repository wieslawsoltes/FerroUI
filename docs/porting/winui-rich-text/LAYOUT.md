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

## Requested-family metrics

Rich layout selects `TextLineMetricsMode::FontFamily` through the existing shared
formatter. `TextParagraphProperties` and `GenericTextParagraphProperties` default
to `GlyphTypeface`; ordinary TextBlock and custom paragraph implementations retain
their existing metric policy. Copying paragraph properties also copies this mode.
`FontFamilyLineMetrics` resolves the regular face of the originally requested
family through FerroUI's FontManager. Character fallback may choose another face
for glyphs without replacing that requested-family provenance.

The pinned
[`CCompositeFontFamily::GetTextLineBoundsMetrics`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/Fonts/CompositeFontFamily.cpp)
and [`LineServicesGetRunTextMetrics`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextServices/TextFormatter/lineservicescallbacks.cpp)
provide the family-box transform. For supplied ascent `A`, descent `D`, gap `G`
and cap height `C`, scaled by the run's effective em size:

| Bounds | Baseline | Height |
|---|---|---|
| Full | `A` | `A + D + G` |
| TrimToCapHeight | `C` | `D + G + C` |
| TrimToBaseline | `A` | `A` |
| Tight | `C` | `C` |

Bounds are applied to each text run before its contribution joins the line.
Nonempty lines have no artificial height floor from a larger paragraph default
font. Empty lines use the default requested family, and break-only markers do not
contribute text metrics. The line retains both the largest individual height and
the aggregate ascent-plus-descent envelope, following the visible contract in
[`LsTextLine::CalculateLineMetrics`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextServices/TextFormatter/LsTextLine.cpp).
The stacking ratio uses the same family-box helper. Explicit rich-edit spacing
still takes precedence after natural metrics are calculated.

Actual glyph faces continue to supply shaping, drawing, decorations and ink.
FontFamily mode keeps runs with different requested families in separate shaping
groups, so a cross-run ligature cannot discard a later family's metric
contribution. Compatible runs with the same requested family still group.
Shaped caches and wrapped/forked continuations retain their metric mode; a
cross-policy request refetches without consuming the older continuation.

For equal supplied ascent `16`, descent `4`, line gap `2` and cap height `11.2`,
the two shared policies produce these boxes:

| Mode | Rich FontFamily baseline / height | Existing TextBlock GlyphTypeface baseline / height |
|---|---|---|
| Full | `16 / 22` | `17 / 22` |
| TrimToCapHeight | `11.2 / 17.2` | `11.2 / 16.2` |
| TrimToBaseline | `16 / 16` | `17 / 17` |
| Tight | `11.2 / 11.2` | `11.2 / 11.2` |

These are source formulas with equal input metrics. They do not establish that
DirectWrite and FerroUI choose identical metrics for the same installed font.

### Embedded objects and measured baselines

The FontFamily path combines full object height, baseline and
height-minus-baseline separately from bounded text, matching the visible
baseline-aligned contribution in
[`InlineObjectHandlers.cpp`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextServices/TextFormatter/InlineObjectHandlers.cpp).
An explicitly smaller line height can still allocate less space than that
natural contribution.

The rich source freezes each embedded control's measured size and baseline.
Later object metrics and arranged child rectangles use the same captured size.
Nested TextBlock, RichTextBlock and RichTextBlockOverflow baselines come from
completed first-line caches; these readers never force nested formatting.
Absent or stale caches fall back to desired height. FerroUI's existing nonzero
attached BaselineOffset override retains precedence, while zero keeps its
existing no-override meaning. A changed baseline with equal desired size retires
the source/layout cache without editing document content or history. The source
contracts are
[`BlockLayoutHelpers::GetElementBaseline`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/BlockLayout/BlockLayoutHelpers.cpp)
and [`PageHostedObjectRun::Format`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/BlockLayout/PageHostedObjectRun.cpp).


### Hosted control measurement constraints

The generic [`ObjectRun::Format`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextServices/inc/ObjectRun.h#L23-L55)
interface accepts remaining-width and pen-position arguments. The concrete
[`PageHostedObjectRun`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/BlockLayout/PageHostedObjectRun.cpp#L35-L124)
used by RichTextBlock reports a fixed size with respect to position within a
line. It ignores those two arguments and measures the child with
`host.GetAvailableMeasureSize()`. The generic signature is therefore not evidence
that a hosted UI control needs to remeasure against the width left after each
preceding run.

The host is the current paragraph fragment. Each RichTextBlock or overflow view
passes its own available size to its page node. The page subtracts padding and
passes the remaining page height to each paragraph; the paragraph's BlockNode
subtracts its collapsed margins and stores that size. ParagraphNode returns
that stored size to hosted controls. It does not reduce the host size after
each formatted line or according to the object's pen position. This chain is
visible in
[`RichTextBlock::MeasureOverride`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlock.cpp#L1326-L1377),
[`RichTextBlockOverflow::MeasureOverride`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextBlock/RichTextBlockOverflow.cpp#L784-L847),
[`PageNode::MeasureCore`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/BlockLayout/PageNode.cpp#L352-L527),
[`BlockNode::Measure`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/BlockLayout/BlockNode.cpp#L61-L120)
and [`ParagraphNode::GetAvailableMeasureSize`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/BlockLayout/ParagraphNode.cpp#L184-L193).

This measurement contract is still partial in FerroUI. The head RichTextBlock
measures embedded controls before constructing the shared rich source, using
the head's inner width and infinite height. That measurement does not subtract
individual paragraph margins or use a later overflow view's own constraints.
The captured metrics keep line and child geometry consistent, but sharing them
across views does not establish the native paragraph/page constraint behavior.
The implementation points are
[`RichTextBlock::measure_override`](../../../src/FerroUI.Controls/rich_text_block.rs),
[`InlineUIContainer::measure_embedded_controls`](../../../src/FerroUI.Controls/documents/inline_ui_container.rs)
and [`RichTextSource`](../../../src/FerroUI.Controls/rich_text_layout.rs).

Native host ownership is a separate boundary: a different host can attach the
control during paragraph measure; when it cannot attach, the object retains its
cached host and metrics. FerroUI currently moves visual children to their
formatted view during arrange. A future paragraph/page measurement integration
must preserve immutable earlier-page metrics and distinguish measure-time host
changes from later geometry reads. It does not require changing the shared
formatter's public drawable-run API merely to match the generic native method
signature.

## Optional fallback scale

The native `MapCharacters` path returns a font face and an independent scale.
[`TextSegment::AnalyzeFonts`](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/dxaml/xcp/core/text/RichTextServices/TextFormatter/TextSegment.cpp)
splits when either changes. The
[DirectWrite API contract](https://learn.microsoft.com/en-us/windows/win32/api/dwrite_2/nf-dwrite_2-idwritefontfallback-mapcharacters)
defines the scale as a multiplier of the returned font's em size.

FerroUI exposes this through the optional
`IFontManagerImpl::font_fallback_mapper()` capability and
`IFontFallbackMapper::try_map_characters()`. An owned `FontFallbackMapping`
retains the actual glyph face, a nonempty logical UTF-16 prefix and a finite
positive scale. FontFamily mode queries the capability. The existing TextBlock
policy and public legacy shapeable-run entry point do not query it.

Configured fallback lists, aliases and keyed embedded/composite families keep
the existing collection resolver. The optional mapper receives eligible system
families. Returned prefixes must end on scalar and grapheme boundaries, cover
the offered glyphs and produce a finite positive em size. A declined or invalid
mapping advances through one complete grapheme using the existing resolver, so
the provider cannot cause a zero-length retry or split a surrogate pair. No
font-collection or cache borrow crosses the provider callback.

`TextRunProperties::font_rendering_em_size()` remains the explicit source size;
`effective_font_rendering_em_size()` multiplies it by `font_scale()`. Glyphs,
requested-family metrics, decorations and character spacing consume that
multiplier once. Shaping groups distinguish actual face identity and scale.
Cached and forked runs retain their mapped snapshot without querying again;
providers use existing font/layout invalidation when their configuration changes.
Standalone symbols respect a supplied run scale but do not initiate a mapping
query. Empty lines have no character prefix to map and use the default family.

**Skia and Vello currently provide only matched faces.** Both inherit the absent
optional capability and continue at scale `1`. There is no separate DirectWrite
font-manager implementation in this tree. Controlled custom-provider regressions
exercise the shared transport, including callback mutation and reentry. Their
presence does not establish a production nonunit-scale provider, a measured
native font/input example, or native pixel equivalence.

## Native metric boundaries

The enum contract is exact. The rich property and complete native layout remain
partial for these reasons:

| Contract | Implemented behavior and remaining boundary |
|---|---|
| Family baseline and gap | Rich FontFamily mode retains the requested regular-family baseline independently of half-gap placement and applies bounds per run. Existing TextBlock keeps GlyphTypeface behavior. Platform family resolution and native rounding remain separate. |
| Fallback glyphs and scale | Requested-family metrics survive actual glyph fallback. Optional face/prefix/scale transport is functional for a supplied mapper; current Skia/Vello providers remain absent and use scale `1`. Native composite matching and platform scale provision remain partial. |
| Cap height | Main uses positive OS/2 sCapHeight, then a TrueType H outline, then a proportional fallback. DirectWrite's internal missing-cap-height policy is not in the public source. |
| Embedded objects | Full baseline-aligned height/ascent/descent and captured nested text-control baselines contribute independently of text trimming. Other baseline alignments keep FerroUI placement; Top/TextTop can extend outside allocation. Native paragraph/page measurement constraints and measure-time host ownership remain partial; the concrete hosted run does not use the generic remaining-width argument. |
| Property inheritance | The existing FerroUI attached property inherits. Native controls have individual, non-inherited TextLineBounds properties. |
| Engine output | The open formulas do not reproduce opaque LineServices, TextDpi quantization, native font rounding, shaping or rasterization. |

The object position model also remains FerroUI's existing single U+FFFC position;
the native hosted-object run uses two text positions. Typography superscript and
subscript are font features in the inspected native source; they are not evidence
for a separate fallback scale. Full pinned source hashes, symbols, mappings and
provider boundaries are recorded in
[`family-metrics-source-map.json`](family-metrics-source-map.json).

## Reproducible verification

[`check-rich-text-line-stacking.py`](../../../scripts/porting/check-rich-text-line-stacking.py)
extracts and compiles the actual pinned C++ function and the current Rust helper.
Its harness supplies only primitive dependencies and a line-height getter.
It compares 960 combinations of strategy, family ratio, line height, natural
height and first/last flags, using relative tolerance `1e-6` or absolute `1e-5`
for native float versus Rust f64. The recorded result and function hashes are in
[`line-stacking-formula.json`](line-stacking-formula.json).

[`check-rich-text-family-metrics.py`](../../../scripts/porting/check-rich-text-family-metrics.py)
compiles the actual pinned family-bounds function and the entire current Rust
`FontFamilyLineMetrics` helper. Its harness supplies resolved family design
metrics through `GetLineMetrics` and primitive Rust dependencies. The matrix has
3,240 cases over design em, ascent, descent, gap, cap ratio, requested size and
all four modes. The native calculation uses float and the Rust helper uses f64;
the accepted absolute error is at most `1e-5 + 1e-6 * abs(native value)`.

The durable script recompiled the pinned C++ function and the current Rust
helper and passed all 3,240 cases. The recorded maximum absolute error is
`8.239746108529289e-6`.
[`family-metrics-formula.json`](family-metrics-formula.json) records that execution,
the full native/helper hashes, extracted-function hash, exact matrix and output
hashes. This is a source-formula result; it does not run platform font lookup,
native controls or rendering.

```sh
python3 scripts/porting/check-rich-text-line-stacking.py \
  --reference /path/to/microsoft-ui-xaml \
  --output docs/porting/winui-rich-text/line-stacking-formula.json
python3 scripts/porting/check-rich-text-family-metrics.py \
  --reference /path/to/microsoft-ui-xaml \
  --output docs/porting/winui-rich-text/family-metrics-formula.json
```

The family-metrics script also accepts `--source-root` for an extracted pinned
source tree, requiring the exact full-file hash, and `--rust-root` for an isolated
FerroUI source tree. It invokes C++ and rustc directly and does not require Cargo.
Neither formula oracle executes font lookup, native LineServices aggregation or
native rendering.

| Framework regressions | Coverage |
|---|---|
| [`font_family_line_metrics_tests.rs`](../../../src/FerroUI.Base/media/text_formatting/font_family_line_metrics_tests.rs), 16 definitions | Four family boxes, per-run envelopes, fallback provenance, empty lines, full object contributions, shaping groups, caches/continuations, draw/ink and legacy default compatibility. |
| [`rich_text_family_metrics_tests.rs`](../../../src/FerroUI.Controls/rich_text_family_metrics_tests.rs), 7 definitions | Real nested controls, cached baseline/size ownership, equal-size baseline changes without document/history mutation, attached overrides and no forced formatting. |
| [`font_fallback_scale_tests.rs`](../../../src/FerroUI.Base/media/text_formatting/font_fallback_scale_tests.rs), 17 definitions | Controlled provider contracts, face/scale grouping, Unicode prefixes, fallback progress, configured collection precedence, effective em, cache ownership and reentry. |
| [`rich_text_fallback_scale_tests.rs`](../../../src/FerroUI.Controls/rich_text_fallback_scale_tests.rs), 3 definitions | Real rich bounds/caret/ranges, source mutation during mapping and preservation of a newer reentrant layout. |
| [`rich_text_line_stacking_tests.rs`](../../../src/FerroUI.Controls/rich_text_line_stacking_tests.rs) and [`rich_text_line_bounds_tests.rs`](../../../src/FerroUI.Controls/rich_text_line_bounds_tests.rs) | Allocation, paragraph fragments, fitting, modes, clipping, fallback, embedded controls, draw/caret/range geometry and explicit spacing. |

The existing `rich_text_document.xaml` corpus fixture exercises all four bounds
modes through runtime and emitted markup. Actual framework gate results belong
in [TESTS.md](TESTS.md). All 43 new metrics/scale regressions passed in the
coordinated full Base (4,470) and Controls (5,251) gates. The source checkpoint
and exact execution counts are in
[framework-family-link-validation.json](framework-family-link-validation.json).
The separate 3,240-case arithmetic gate and these framework tests do not establish
native-pixel equivalence.
