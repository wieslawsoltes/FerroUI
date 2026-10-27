# Pretext text engine port

## Source and delivered scope

Source: [PretextSharp](https://github.com/wieslawsoltes/PretextSharp/tree/e70a544e43f7ecffee76d2d22a0224213d7fbba3),
commit `e70a544e43f7ecffee76d2d22a0224213d7fbba3`, MIT.
FerroUI starting commit: `500095d6a688a1077dfaf1e06bfa0d3a6f7fbbb8`.
The [draft PR](https://github.com/wieslawsoltes/FerroUI/pull/346) was opened with the
source baseline before implementation.

The port follows [PORTING-GUIDE.md](PORTING-GUIDE.md): source-level traceability,
product behavior, idiomatic ownership, safe Rust, framework service reuse and
matching behavioral tests. Four reusable crates provide the portable contracts,
engine, layout helpers and native integration. The optional `pretext` facade
feature exposes all three functional layers through `ferroui`.

| Crate | Responsibility |
| --- | --- |
| `ferroui-pretext-contracts` | Font descriptors, measurement/shaping traits, immutable glyph output and errors; no dependencies. |
| `ferroui-pretext` | Preparation, Unicode segmentation, cached measurements, line breaking, rich inline flow, prepared shaping and profiling. |
| `ferroui-pretext-layout` | Wrap metrics, tight-width search, obstacle geometry and multi-column flow. |
| `ferroui-pretext-media` | FerroUI font fallback, bidi, native shaping, actual font faces and glyph drawing. |

Uno controls and the Uno/AppKit application hosts are outside this request. C#
DirectWrite, CoreText, FreeType and SkiaSharp wrappers are replaced by FerroUI's
existing font/shaping services. No independent native text library, CLR runtime
layer or sample application shell is introduced.

## File inventory

Paths in the source column are relative to the pinned source repository. Paths
in the Rust column are relative to `src/Pretext/`.

| Source | Rust | Disposition |
| --- | --- | --- |
| `src/Pretext.Contracts/TextMeasurementContracts.cs` | `FerroUI.Pretext.Contracts/text_measurement_contracts.rs` | Measurement/factory contracts complete; assembly attributes replaced by explicit registration. |
| `src/Pretext.Contracts/TextShapingContracts.cs` | `FerroUI.Pretext.Contracts/text_shaping_contracts.rs` | Direction/options/glyph/font-run/shaper contracts complete; owned immutable data and value snapshots. |
| `src/Pretext.Contracts/PretextFontDescriptor.cs` | `FerroUI.Pretext.Contracts/pretext_font_descriptor.rs` | Descriptor, permissive font parser and generic aliases ported. |
| `src/Pretext/PretextLayout.cs` | `FerroUI.Pretext/pretext_layout.rs`, `pretext_layout_backends.rs` and public methods beside their algorithms | Models and entry points complete; partial-class members grouped with implementations. |
| `src/Pretext/PretextLayout.Analysis.cs` | `FerroUI.Pretext/pretext_layout_analysis.rs` | Normalization and all token/word/punctuation/URL/numeric/CJK merge passes ported. |
| `src/Pretext/PretextLayout.Locale.cs` | `FerroUI.Pretext/pretext_layout_locale.rs` | Product segmentation retained with compiled ICU4X dictionaries; native ICU loading omitted. |
| `src/Pretext/PretextLayout.Measurement.cs` | `FerroUI.Pretext/pretext_layout_measurement.rs` | Cached font and segment measurement, contextual prefix widths and CJK units ported. |
| `src/Pretext/PretextLayout.Preparation.cs` | `FerroUI.Pretext/pretext_layout_preparation.rs` | Prepared metrics, retained source segments, chunks and preparation profiling ported. |
| `src/Pretext/PretextLayout.Bidi.cs` | `FerroUI.Pretext/pretext_layout_bidi.rs` | Source first-strong and weak/neutral/implicit level behavior retained. |
| `src/Pretext/Generated/PretextBidiData.cs` | `FerroUI.Pretext/generated/pretext_bidi_data.rs` | Exact source Unicode 17 classification data, including 714 ranges. |
| `src/Pretext/PretextLayout.LineBreak.cs` | `FerroUI.Pretext/pretext_layout_line_break.rs` | Simple/general batch and streaming paths, tabs, hanging whitespace and discretionary breaks ported. |
| `src/Pretext/PretextLayout.LineText.cs` | `FerroUI.Pretext/pretext_layout_line_text.rs` | Lazy materialization and source discretionary-hyphen placement ported. |
| `src/Pretext/PretextLayout.RichInline.cs` | `FerroUI.Pretext/pretext_layout_rich_inline.rs` | Mixed fonts, boundary spaces, atomic items, extra widths and fragment streaming ported. |
| `src/Pretext/PretextLayout.Shaping.cs` | `FerroUI.Pretext/pretext_layout_shaping.rs` | Whole-text and prepared caches, safe slices and context-sensitive reshaping ported. |
| `src/Pretext/PretextLayout.Backends.cs` | `FerroUI.Pretext/pretext_layout_backends.rs` | Explicit factories, support/priority selection and overrides; reflection discovery intentionally omitted. |
| `src/Pretext.Layout/LayoutPrimitives.cs` | `FerroUI.Pretext.Layout/layout_primitives.rs` | All data records ported. |
| `src/Pretext.Layout/PreparedTextMetrics.cs` | `FerroUI.Pretext.Layout/prepared_text_metrics.rs` | All sizing helpers ported; natural width uses infinity. |
| `src/Pretext.Layout/ObstacleLayoutHelper.cs` | `FerroUI.Pretext.Layout/obstacle_layout_helper.rs` | Interval, circle, ellipse and rectangle helpers ported. |
| `src/Pretext.Layout/ColumnFlowLayout.cs` | `FerroUI.Pretext.Layout/column_flow_layout.rs` | Column flow ported with the source defects corrected below. |
| C# native measurement/shaping backend projects | `FerroUI.Pretext.Media/ferro_text_measurer_factory.rs`, `ferro_shaped_text.rs` | Framework-native replacement; individual backend implementations intentionally not duplicated. |

The upstream test directory is named `tests/Pretext.Uno.Tests`, but its preparation,
layout, rich-inline and most shaping tests exercise the portable engine. Their
Rust counterparts are `preparation_tests.rs`, `layout_tests.rs`,
`rich_inline_tests.rs`, `shaping_tests.rs` and the native adapter's `tests.rs`.

All 102 upstream test scenarios are represented or adapted; this is not a
one-to-one count of Rust test functions because the two discovery scenarios
share one explicit-registration test.

| Upstream scenarios | Rust coverage and adaptation |
| --- | --- |
| 49 preparation tests | `preparation_tests.rs`; source assertions retained, with additional Unicode and error regressions. |
| 37 layout tests | `layout_tests.rs`; batch, geometry, streaming, preserved whitespace, CJK and discretionary breaks. |
| 1 rich-inline test | `rich_inline_tests.rs`; expanded with atomic, zero-advance and fragment regressions. |
| 12 general shaping tests | `shaping_tests.rs`; factory overrides, cached preparation, exact-line shaping and safe glyph slices. |
| 1 SkiaSharp UTF-16 test | `shaping_contract_reports_clusters_as_utf16_text_indices` and native `font_fallback_preserves_actual_faces_and_utf16_clusters`; use framework services instead of the excluded wrapper. |
| 1 shaping and 1 measurement auto-discovery test | `registered_backend_selection_uses_supported_priority_and_combined_shaping` and native combined-factory tests; explicit supported/priority registration replaces assembly scanning. |

## API translations

- C#'s static `PretextLayout` becomes an explicit Rust engine for preparation,
  backend selection and shaping. Stateless layout/metrics operations remain
  associated functions. Independent documents and hosts can use different
  backends without changing process-wide state.
- `PreparedTextWithSegments` owns `PreparedText` and dereferences to its metrics.
  Prepared retained-text values use `Rc`; plain records use value semantics.
  Arrays are exposed as immutable slices.
- `LayoutCursor` and rich cursors use nonnegative segment/grapheme indices.
  Glyph clusters remain UTF-16 code-unit indices, matching FerroUI's text service
  contract. Internal string storage is UTF-8 with cached checked byte boundaries.
- Nullable source return values become `Option`; fallible preparation, backend
  creation and shaped materialization return `Result<_, PretextError>`.
- Source optional parameters are explicit Rust values. `PrepareOptions::default`
  and `PretextShapeOptions::default` preserve the corresponding source defaults.
- `SetTextMeasurerFactory(null)`/assembly discovery are replaced by a configured
  engine or `from_factories`; `FerroTextMeasurerFactory::try_current` connects the
  application services. An explicit shaper override survives measurer changes.
- `IDisposable` becomes ordinary Rust ownership. No reflection, BCL collections,
  weak-table runtime emulation, assembly attributes or target-framework polyfills
  ship in the runtime.
- `ProfilePrepare` takes a caller-supplied monotonic millisecond clock. This
  preserves diagnostics on native and browser hosts without `Instant` in the
  core. The clock is validated for finite monotonic samples.

## Runtime and integration details

Preparation retains separate fit/paint advances, contextual grapheme prefix
widths and hard-break chunks. Geometry-only walking does not allocate strings;
whole-segment materialization avoids building grapheme caches. Chunk lookup uses
binary search. Normal whitespace, preserved spaces, eight-space tab stops,
nonbreaking glue, zero-width breaks, soft hyphens and CJK KeepAll behavior retain
the source's distinctions, including its tested partial-segment hyphen placement.

Prepared shaping retains a backend snapshot. Whole safe ranges reuse glyph data;
partial or invisible-break boundaries shape the exact materialized line. The
engine caches a payload without owning its source document, while callers retain
source plus payload in a small wrapper. This preserves cache reuse across rich
inline/convenience calls without an ownership cycle. Changing a backend or
clearing an engine's caches cannot invalidate retained prepared metrics or
shaped values. Backend callbacks and final destructors run outside `RefCell`
registry/cache borrows, and reentrant replacements cannot insert stale state.

The native adapter uses FerroUI's `BidiData`/`BidiAlgorithm`,
`TextCharacters::get_shapeable_characters`, `TextShaper` and `BidiReorderer`.
Measurement and glyph output therefore use the same actual shaping and font
fallback pipeline. Portable output records resolved font identities and UTF-16
clusters; `FerroShapedText` retains real `GlyphTypeface`/`GlyphRun` handles for
baseline-relative drawing. The measurement path avoids allocating portable
glyph records and drawing resources.

The adapter checks `FontManager`, cached `TextShaper` and raw `ITextShaperImpl`
identity. The last check is necessary because fallback glyph faces create their
backend-specific shaper faces lazily. An incomplete registration or raw-only
backend replacement returns an error before it can mix incompatible services.

## Intentional differences and source corrections

1. The default content locale is invariant, rather than inherited from the
   process culture. `set_locale` supplies a language explicitly. ICU4X compiled
   data works on native and WebAssembly hosts without native ICU discovery.
   Reference cases specify `en` on both engines to avoid host POSIX-locale
   tailoring; explicit Thai behavior is separately covered by source tests.
   Compatibility adapters preserve native ICU colon rules and dictionary-script
   transition context using Unicode properties. A shared text-element iterator
   removes only GB9c conjunct joins to preserve the source .NET `StringInfo`
   cursor boundaries while retaining the current emoji, combining, RI and
   Hangul rules. Dictionary data may vary between native ICU versions outside
   the tested corpus. ICU dependencies use exact version pins, and the checked-in
   `Cargo.lock` records the complete resolved dependency set.
2. Invalid measured widths and overflowing tab stops return errors. Malformed
   cursors do not index outside arrays, and invalid shaped ranges return errors.
3. Retained shaped values keep the original backend after reconfiguration; they
   do not silently combine old metrics with a new shaping service.
4. Fully blocked column rows do not consume source text. The source's nullable
   `FirstOrDefault` over a value type incorrectly produced a zero-length slot.
5. Circle exclusion uses the full line band, rather than only its midpoint.
6. Natural-width measurement uses true infinity instead of `100_000`.
7. Empty slot selection returns `None`. Column flow and tight-width search
   validate finite geometry and progress, and skip columns that cannot fit a row.
8. A rich line containing actual zero-advance text is retained. Source stepping
   used width zero as an absence sentinel and could discard that content.
9. When supported registered factories tie for highest priority, the first
   registration wins. Source assembly discovery throws on a tie. Explicit
   ordered registration makes the Rust choice deterministic and inspectable.

The preparation bidi metadata intentionally retains the source's limited
algorithm; it is not a replacement for full paragraph visual reordering.
FerroUI's native shaping path supplies the complete framework bidi behavior.

## Explicit exclusions

- `Pretext.Uno` controls (`StretchScrollHost`, `UiRenderScheduler`) and Uno hosts.
- AppKit/Uno shell windows, sample pages, application resources and platform
  packaging. Reusable algorithm examples are supplied as Rust examples instead.
- DirectWrite/CoreText/FreeType/SkiaSharp C# FFI wrappers and their native-library
  loaders. Native font/rendering support is provided by the host FerroUI backend.
- `PretextCompatibility.cs`, `Polyfills/*`, `IsExternalInit.cs`, assembly metadata
  and generated .NET record/overload infrastructure.
- NuGet, target-framework build logic and .NET release tooling.

No public portable algorithm is represented by an empty implementation or a
`todo!()` placeholder.

## Validation and reproducible source oracle

The source generator compiles the **unchanged** pinned C# contracts and core,
with the source's delegate backend. The fixture corpus covers 240 deterministic
and mixed-script cases, multiple widths, both whitespace modes and KeepAll.
It compares prepared segments, kinds, widths, contextual prefix arrays, bidi
levels, chunks, natural width, aggregate metrics, batch lines and streaming
lines/cursors. Rust tests also verify that ranges reconstruct the normalized
source and that metric-only preparation agrees with retained-text preparation.
The source remains a development oracle; no .NET or Uno runtime is shipped.

See [reference tooling](../../scripts/pretext/README.md) for regeneration and
source-cleanliness verification. The dedicated `pretext.yml` workflow checks
Rust 1.89 and stable, formatting, all four crates' tests, Clippy, public docs,
examples, facade integration and WebAssembly compilation.

### Verified results

| Check | Result |
| --- | --- |
| Unit and integration tests on Rust 1.89 | 165 passed: 3 contracts, 133 core, 15 layout helpers and 14 native adapter tests. |
| Unchanged-source corpus | All 240 cases match; generator `--check` also verifies reproducibility against the clean pinned source. |
| Formatting | All four crates pass `cargo fmt --check`. |
| Clippy | Libraries, tests and examples in all four crates pass with `--no-deps -- -D warnings`. |
| Public API documentation | All four crates pass `cargo doc --no-deps` with `RUSTDOCFLAGS='-D warnings'`. |
| WebAssembly | All four crates compile for `wasm32-unknown-unknown`. |
| Facade integration | `ferroui --no-default-features --features pretext` compiles. |
| Runnable examples | `editorial` and `prepared_labels` run successfully, covering changing widths, obstacle flow, atomic items and native glyph drawing. |
| Release metadata | 75 published workspace crates at 0.4.0 pass metadata, version and notice checks, including the four new crates. |

Clippy's `--no-deps` confines warning errors to the selected packages; existing
workspace dependency warnings are outside this port. WebAssembly validation is
a compile check, not a browser rendering test.

### Commands

```sh
cargo test --locked -p ferroui-pretext-contracts -p ferroui-pretext \
  -p ferroui-pretext-layout -p ferroui-pretext-media
cargo clippy --locked --all-targets --no-deps -p ferroui-pretext-contracts -p ferroui-pretext \
  -p ferroui-pretext-layout -p ferroui-pretext-media -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps \
  -p ferroui-pretext-contracts -p ferroui-pretext \
  -p ferroui-pretext-layout -p ferroui-pretext-media
cargo check --locked --target wasm32-unknown-unknown \
  -p ferroui-pretext-contracts -p ferroui-pretext -p ferroui-pretext-layout \
  -p ferroui-pretext-media
cargo check --locked -p ferroui --no-default-features --features pretext
cargo run --locked -p ferroui-pretext-layout --example editorial
cargo run --locked -p ferroui-pretext-media --example prepared_labels
python3 scripts/pretext/generate-reference.py --source ../PretextSharp --check
python3 scripts/release/crates.py check
```

Native validation uses FerroUI's deterministic font/shaper/render test services,
including comparison with its text formatter. It does not claim pixel parity
against installed operating-system fonts or browser rendering on a real device.

## Attribution

Each new crate carries the source copyright and full MIT text in `NOTICE.md`,
plus the repository license and project attribution links required by release
checks. The source credits Cheng Lou's original
[pretext](https://github.com/chenglou/pretext); that provenance is preserved.
