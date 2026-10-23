# Typography properties, snapshots and shaping

The port implements all 43 public `Typography` attached-property families through
FerroUI's property, document, formatter and font-feature APIs. The exact public
member mappings remain partial because implementation of the property surface
does not establish every native font, layout or rasterizer outcome. The seven
enum types and their 38 numeric constants have separate value-contract mappings.

## Pinned source and Rust responsibilities

The source reference is
[`fe244ca6`](https://github.com/microsoft/microsoft-ui-xaml/tree/fe244ca6ac4d482712b234a9de3cf771cc6481de).

| Pinned source role | Rust implementation |
|---|---|
| `core/text/common/Typography.cpp`: constructor defaults and feature-list generation | `documents/typography.rs` registers inherited attached properties; `documents/typography_format.rs` captures, overlays and resolves their values and generates ordered `FontFeature` records. |
| `XamlOM/Model/Microsoft.UI.Xaml.Documents.cs`: property declarations, getters and setters | `Typography` is a static FerroUI type with typed property accessors, all 86 public get/set markup methods and normal property inheritance. |
| `XamlOM/Model/Microsoft.UI.Xaml.cs`: seven enum declarations | `FerroUI.Base/media/font_typography.rs` retains the exact values and registers the types in `FerroUI.Media`. |
| Native property/layout/shaping plumbing | Existing `TextElement`, `Inline`, `CharacterFormat`, `TextBlock`, `SelectableTextBlock`, `TextPresenter`, rich layout/overflow and `TextFormatter` consumers; existing HarfBuzz shaping receives the final feature sequence. |

The source files, public XML identifiers, target symbols and individual test
anchors are retained in `mapping.json`, `inventory.json` and `API-MAP.md`.
Attribution is recorded next to both the Base enum and Controls document code.

## Property and OpenType feature mapping

Each property inherits. The four default-on booleans emit an explicit zero when
disabled. Other booleans enable the listed feature when true; integer selectors
emit their value when nonzero. Enum choices select the feature or features in the
following table. `Normal` does not enable an additional feature.

| Property | Rust value | Default | Feature family |
|---|---|---|---|
| `AnnotationAlternates` | `i32` | `0` | `nalt` |
| `EastAsianExpertForms` | `bool` | `false` | `expt` |
| `EastAsianLanguage` | `FontEastAsianLanguage` | `FontEastAsianLanguage::Normal` | Enum choices below |
| `EastAsianWidths` | `FontEastAsianWidths` | `FontEastAsianWidths::Normal` | Enum choices below |
| `StandardLigatures` | `bool` | `true` | `liga` |
| `ContextualLigatures` | `bool` | `true` | `clig` |
| `DiscretionaryLigatures` | `bool` | `false` | `dlig` |
| `HistoricalLigatures` | `bool` | `false` | `hlig` |
| `StandardSwashes` | `i32` | `0` | `swsh` |
| `ContextualSwashes` | `i32` | `0` | `cswh` |
| `ContextualAlternates` | `bool` | `true` | `calt` |
| `StylisticAlternates` | `i32` | `0` | `salt` |
| `StylisticSet1` | `bool` | `false` | `ss01` |
| `StylisticSet2` | `bool` | `false` | `ss02` |
| `StylisticSet3` | `bool` | `false` | `ss03` |
| `StylisticSet4` | `bool` | `false` | `ss04` |
| `StylisticSet5` | `bool` | `false` | `ss05` |
| `StylisticSet6` | `bool` | `false` | `ss06` |
| `StylisticSet7` | `bool` | `false` | `ss07` |
| `StylisticSet8` | `bool` | `false` | `ss08` |
| `StylisticSet9` | `bool` | `false` | `ss09` |
| `StylisticSet10` | `bool` | `false` | `ss10` |
| `StylisticSet11` | `bool` | `false` | `ss11` |
| `StylisticSet12` | `bool` | `false` | `ss12` |
| `StylisticSet13` | `bool` | `false` | `ss13` |
| `StylisticSet14` | `bool` | `false` | `ss14` |
| `StylisticSet15` | `bool` | `false` | `ss15` |
| `StylisticSet16` | `bool` | `false` | `ss16` |
| `StylisticSet17` | `bool` | `false` | `ss17` |
| `StylisticSet18` | `bool` | `false` | `ss18` |
| `StylisticSet19` | `bool` | `false` | `ss19` |
| `StylisticSet20` | `bool` | `false` | `ss20` |
| `Capitals` | `FontCapitals` | `FontCapitals::Normal` | Enum choices below |
| `CapitalSpacing` | `bool` | `false` | `cpsp` |
| `Kerning` | `bool` | `true` | `kern` |
| `CaseSensitiveForms` | `bool` | `false` | `case` |
| `HistoricalForms` | `bool` | `false` | `hist` |
| `Fraction` | `FontFraction` | `FontFraction::Normal` | Enum choices below |
| `NumeralStyle` | `FontNumeralStyle` | `FontNumeralStyle::Normal` | Enum choices below |
| `NumeralAlignment` | `FontNumeralAlignment` | `FontNumeralAlignment::Normal` | Enum choices below |
| `SlashedZero` | `bool` | `false` | `zero` |
| `MathematicalGreek` | `bool` | `false` | `mgrk` |
| `Variants` | `FontVariants` | `FontVariants::Normal` | Enum choices below |

| Enum | Nondefault choice → OpenType tag(s) |
|---|---|
| `FontEastAsianLanguage` | `HojoKanji` → `hojo`; `Jis04` → `jp04`; `Jis78` → `jp78`; `Jis83` → `jp83`; `Jis90` → `jp90`; `NlcKanji` → `nlck`; `Simplified` → `smpl`; `Traditional` → `trad`; `TraditionalNames` → `tnam`. |
| `FontEastAsianWidths` | `Full` → `fwid`; `Half` → `half`; `Proportional` → `pwid`; `Quarter` → `qwid`; `Third` → `twid`. |
| `FontCapitals` | `AllSmallCaps` → `c2sc`, `smcp`; `SmallCaps` → `smcp`; `AllPetiteCaps` → `c2pc`, `pcap`; `PetiteCaps` → `pcap`; `Unicase` → `unic`; `Titling` → `titl`. |
| `FontFraction` | `Stacked` → `afrc`; `Slashed` → `frac`. |
| `FontNumeralStyle` | `Lining` → `lnum`; `OldStyle` → `onum`. |
| `FontNumeralAlignment` | `Proportional` → `pnum`; `Tabular` → `tnum`. |
| `FontVariants` | `Superscript` → `sups`; `Subscript` → `subs`; `Ordinal` → `ordn`; `Inferior` → `sinf`; `Ruby` → `ruby`. |

Integer selectors retain their signed `i32` parameter values, as the source
projection does. The shaping backend receives the same bit representation used
by existing raw font features. Actual glyph availability belongs to the chosen
font; the port does not synthesize an unavailable alternate or rewrite document
text to imitate a feature.

## Local intent, inheritance and history

`TypographyFormat` contains one optional field per property. `None` means that
the snapshot/patch does not assign that property. An explicit default such as
`Some(FontCapitals::Normal)` or `Some(false)` remains an assignment. This lets a
nested inline reset an inherited nondefault value and retain that reset after a
split, a structured copy, a snapshot restore or undo/redo.

`CharacterFormat::overlay` overlays typography field by field. Changing one
family preserves independent choices in other families, alongside the existing
font, brush, decoration and raw-feature fields. Effective presentation resolves
the property system before generating features. An inherited host/style change
invalidates layout without becoming a document edit or cancelling composition;
a local document-node property assignment participates in document mutation and
history normally.

The runtime and compiled markup fixture is
`tests/FerroUI.Markup.Xaml.UnitTests/emitter/fixtures/rich_typography.xaml`.
For example:

```xml
<RichTextBlock xmlns="https://github.com/ferroui"
               Typography.Kerning="False"
               Typography.Capitals="SmallCaps">
  <Paragraph>
    <Run Text="Inherited small capitals" />
    <Run Text=" and an explicit reset"
         Typography.Capitals="Normal"
         Typography.Kerning="True" />
  </Paragraph>
</RichTextBlock>
```

## Raw features and formatter integration

The existing `FontFeatures` collection is a FerroUI extension to the named
property surface. Derived named features come first. Every raw feature is then
appended in its original order with its value and start/end range intact,
including duplicate tags. This preserves the existing shaper's last-override
semantics instead of reducing a sequence to a dictionary. When all named
properties resolve to their defaults, the original raw collection is returned
without replacing its identity.

Shaping compatibility compares complete ordered feature values and ranges.
Equivalent independently allocated collections can share shaping work; genuinely
different feature inputs cannot be coalesced across a run boundary. Selected
runs and preedit runs receive their actual inline features. `SelectableTextBlock`
creates its inline runs before constructing selected-run overrides, preserving
an explicit reset on the first selected layout.

Weak collection subscriptions observe mutable raw features in plain text,
presenters and empty rich views. Replacing a collection or dropping its owner
releases those subscriptions. This invalidation path does not create text-history
entries or end active preedit.

## RTF and verification boundaries

The current RTF codec has no lossless representation for these nondefault named
typography features. It rejects such output with its typed unsupported-format
error. Explicit assignments whose resolved values are all defaults can be
exported; they remain explicit in in-process snapshots. This distinction avoids
both silent formatting loss and rejection of ordinary default-formatted text.

The focused suites cover all property defaults/accessors, all enum values and
tag arms, inheritance and explicit resets, ordered raw overrides, formatting
patches, rich splits/history, mutable-feature lifetimes, runtime markup and the
compiled fixture. Two HarfBuzz cases use the repository's real Inter font to
verify kerning positions and slashed-zero glyph changes, including scoped raw
overrides and source clusters. Their completed execution evidence is recorded
in `TESTS.md`; test definitions alone are not a completed gate.

The independent static audit reads the pinned Git objects and compares the
declarations and feature projection directly:

```sh
python3 scripts/porting/check-rich-text-typography.py --reference /path/to/microsoft-ui-xaml
```

## Reusing the existing font weights

WinUI's eleven `FontWeights` properties already have exact counterparts in
FerroUI's `media::FontWeight`. The port reuses those associated constants, typed
markup fields and named parser entries. A duplicate static provider would add
another API without adding text functionality.

| WinUI property / FerroUI constant | Numeric weight |
|---|---:|
| `Thin` | 100 |
| `ExtraLight` | 200 |
| `Light` | 300 |
| `SemiLight` | 350 |
| `Normal` | 400 |
| `Medium` | 500 |
| `SemiBold` | 600 |
| `Bold` | 700 |
| `ExtraBold` | 800 |
| `Black` | 900 |
| `ExtraBlack` | 950 |

The independent audit reads the pinned `Microsoft.UI.Text.xml` Git blob and
compares every number with the existing Rust constant, metadata field and named
parser entry. It also verifies the representative runtime-test anchors for
conversion, registration and compiled/runtime markup:

```sh
python3 scripts/porting/check-rich-text-font-weights.py --reference /path/to/microsoft-ui-xaml
```

All eleven named value contracts are mapped complete. The static provider type
remains partial: there is no `Microsoft.UI.Text.FontWeights` namespace alias or
WinRT activation/ABI implementation. Existing Rust tests do not individually
compare all eleven names with WinUI; that exhaustive comparison belongs to the
static audit. FerroUI's open `i32` wrapper and numeric parsing retain their
existing behavior. Native validation, font selection and raster metrics are
separate contracts.

It checks 43 inherited defaults and field operations, 86 markup accessors,
38 enum values/projection arms, 69 ordered feature-add sites, four signed
selectors and explicit raw-feature override retention. It is a source audit,
not a font-support, runtime, native-appearance or performance test. Remaining
conformance boundaries are explicit in RT-I25 and RT-P01/RT-P06.
