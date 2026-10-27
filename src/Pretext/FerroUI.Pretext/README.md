# ferroui-pretext

Prepare text once, then reuse its measurements for wrapping, streaming geometry,
rich inline flow and glyph output. This is the safe Rust port of PretextSharp's
portable engine, integrated with FerroUI through a separate text-service adapter.

## Native FerroUI use

After configuring the application's normal font and shaping backends:

```rust
use ferroui_pretext::{PrepareOptions, PretextLayout};
use ferroui_pretext_media::FerroTextMeasurerFactory;

let engine = FerroTextMeasurerFactory::try_current()?.create_layout();
let prepared = engine.prepare_with_segments(
    "Prepare once. Reflow without measuring the text again.",
    "16px sans-serif",
    PrepareOptions::default(),
)?;

let narrow = PretextLayout::layout_with_lines(&prepared, 180.0, 22.0);
let wide = PretextLayout::layout_with_lines(&prepared, 360.0, 22.0);
```

The optional `pretext` feature on the `ferroui` facade reexports this crate as
`ferroui::pretext`, the layout helpers as `ferroui::pretext_layout`, and the native
adapter as `ferroui::pretext_media`.

## Portable custom backends

The engine has no renderer or framework dependency. A measurement factory can
wrap an existing typography service:

```rust
use std::rc::Rc;
use ferroui_pretext::{DelegateTextMeasurerFactory, PrepareOptions, PretextLayout};

// A deterministic example measurer. Production typography should use whole-run
// shaping, including contextual advances and fallback fonts.
let engine = PretextLayout::new(Rc::new(DelegateTextMeasurerFactory::new(
    |text: &str, _: &str| text.chars().count() as f64 * 8.0,
)));
let prepared = engine.prepare_with_segments(
    "one two three", "16px Example", PrepareOptions::default(),
)?;
let metrics = PretextLayout::layout(&prepared, 64.0, 20.0);
```

Factories and caches belong to an engine instance. There is no process-wide
backend registration. Implement `IPretextTextMeasurerFactory` and optionally
`IPretextTextShaperFactory` for a custom renderer. Combined factories expose
`shaper_factory`; an explicit shaping override survives measurement changes.

## API selection

| Need | API |
| --- | --- |
| Reusable metrics without retained text | `prepare` |
| Source segments, bidi levels and lazy lines | `prepare_with_segments` |
| Count and height | `PretextLayout::layout` |
| Materialized text and geometry | `layout_with_lines` |
| Variable-width custom flow | `layout_next_line_range` / `layout_next_line` |
| Allocation-free geometry walking | `walk_line_ranges` |
| Delayed text materialization | `materialize_line_range` |
| Widest line or natural width | `measure_line_stats` / `measure_natural_width` |
| Mixed fonts and atomic inline badges | `prepare_rich_inline` and rich range APIs |
| Glyph ids, positions, clusters and font runs | `shape_text` / `try_shape_text` |
| Repeated glyph materialization | `shape_prepared_text` / `materialize_shaped_line_range` |
| Profiling with a native or browser clock | `profile_prepare` with a monotonic millisecond callback |

`WhiteSpaceMode::Normal` collapses ordinary ASCII whitespace and trims boundaries.
`PreWrap` preserves spaces, tab stops and hard breaks. The engine supports
nonbreaking glue, zero-width breaks, discretionary hyphens, CJK punctuation and
`WordBreakMode::KeepAll`. Empty normalized input has zero lines. Preserved trailing
whitespace can paint beyond the fit width, matching the source contract.

Layout cursors count **segments and graphemes**. Glyph clusters count **UTF-16
code units**, matching FerroUI text services. Rust strings remain UTF-8; all
slicing uses checked grapheme byte boundaries.

## Lifetime and caching

Prepared values retain their immutable measurements when an engine's backend or
locale changes. Prepared shaped values also retain the backend that created them.
Whole safe segments reuse the prepared glyph run; partial or context-sensitive
boundaries reshape the exact materialized line and cache the result.

A cache payload retains glyph data without owning the source document. Repeated
rich/convenience calls reuse that payload even when their temporary wrappers are
dropped. A prepared-shaping cache miss prunes dead source keys, and `clear_cache`
drops all engine caches. Backend callbacks and destructors execute without a
registry or cache borrow held.

## Unicode and source compatibility

Word segmentation uses ICU4X compiled dictionaries on native and WebAssembly
hosts. Call `set_locale(Some("th"))`, for example, to supply a content language.
The default is invariant, independent of process culture. The preparation bidi
metadata preserves the source algorithm; native rendering uses FerroUI's full
bidi and font fallback services.

The port report records exact source mappings, compatibility tailoring, deliberate
source bug fixes and validation evidence:
[docs/porting/pretext.md](https://github.com/wieslawsoltes/FerroUI/blob/main/docs/porting/pretext.md).

```sh
cargo test -p ferroui-pretext
cargo run -p ferroui-pretext-layout --example editorial
cargo run -p ferroui-pretext-media --example prepared_labels
```
