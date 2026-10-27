# FerroUI Pretext Layout

Platform-neutral sizing and obstacle-aware flow for prepared text. Preparation
and text measurement come from `ferroui-pretext`; this crate adds no renderer,
native backend, thread, clock or file dependency.

## APIs

| API | Purpose |
| --- | --- |
| `PreparedTextMetrics::measure_max_line_width` | Measure genuinely unwrapped text, honoring preserved hard breaks. |
| `PreparedTextMetrics::collect_wrap_metrics` | Measure count, height and widest painted line without materializing strings. |
| `PreparedTextMetrics::find_tight_wrap_metrics` | Find the smallest integral width preserving the original line count. |
| `ObstacleLayoutHelper::carve_text_line_slots` | Subtract overlapping intervals from an available horizontal line slot. |
| `ObstacleLayoutHelper::circle_interval_for_band` | Compute circle exclusion over an entire text band. |
| `ObstacleLayoutHelper::ellipse_interval_for_band` | Compute ellipse exclusion with independent radii and padding. |
| `ObstacleLayoutHelper::get_rect_intervals_for_band` | Select rectangle exclusions intersecting a text band. |
| `ObstacleLayoutHelper::pick_slot` | Select the widest slot, with an explicit tie direction. |
| `ColumnFlowLayout::layout_into_columns` | Flow text through ordered rectangular columns around rectangles and circles. |

Coordinates and widths use the configured measurer's units. Circle `x` and `y`
are its center. Band boundaries are half open: tangent obstacles do not block
a row. Column flow uses one widest slot on each row; remaining text beyond the
last column is not materialized.

```rust
use std::rc::Rc;
use ferroui_pretext::{DelegateTextMeasurerFactory, PrepareOptions, PretextLayout};
use ferroui_pretext_layout::{ColumnFlowLayout, RectObstacle};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
// Use the native FerroUI adapter in a real application. This deterministic
// measurer keeps the example independent of installed fonts.
let engine = PretextLayout::new(Rc::new(DelegateTextMeasurerFactory::new(
    |text: &str, _: &str| text.chars().count() as f64 * 8.0,
)));
let prepared = engine.prepare_with_segments(
    "Prepare once and flow across multiple columns.",
    "16px Inter",
    PrepareOptions::default(),
)?;
let columns = [
    RectObstacle { x: 0.0, y: 0.0, width: 160.0, height: 40.0 },
    RectObstacle { x: 180.0, y: 0.0, width: 160.0, height: 40.0 },
];
let lines = ColumnFlowLayout::layout_into_columns(
    &prepared, &columns, &[], &[], 20.0, 20.0,
)?;
for line in lines {
    println!("{} at ({}, {})", line.text, line.x, line.y);
}
# Ok(())
# }
```

## Source mapping and deliberate corrections

All four source files from `Pretext.Layout` are mapped one to one:

| Source | Rust |
| --- | --- |
| `LayoutPrimitives.cs` | `layout_primitives.rs` |
| `PreparedTextMetrics.cs` | `prepared_text_metrics.rs` |
| `ObstacleLayoutHelper.cs` | `obstacle_layout_helper.rs` |
| `ColumnFlowLayout.cs` | `column_flow_layout.rs` |

The source's `IsExternalInit.cs` polyfill is unnecessary in Rust. C# optional
parameters are explicit Rust arguments. Plain data records have public fields
and derived value equality; stateless classes retain associated functions.

This port corrects the following observed source defects:

- No-fitting-slot selection returns `None`. The source's value-type
  `FirstOrDefault()` produced `(0, 0)` and consumed text in blocked rows.
- Circle exclusion in column flow uses the entire row band, rather than its
  midpoint, so glyph rectangles do not overlap a circle near a row edge.
- Maximum line width uses infinity rather than a magic `100_000` width.
- Empty slot selection returns `None` rather than indexing an absent element.
- Column flow rejects nonpositive/nonfinite line heights, invalid geometry and
  coordinates that cannot advance to another finite row. Tight-wrap search
  rejects invalid widths and bounds the search to exactly representable `f64`
  integers. Both return `LayoutError` instead of looping indefinitely.
- Columns with no possible slot (zero width, narrower than the minimum slot,
  too short, or completely covered by one rectangle) are skipped immediately,
  avoiding a row-by-row scan for tiny positive line heights.
- Geometry helpers ignore degenerate/nonfinite blocked intervals, return no
  intersection for invalid shapes, and reject invalid base slots and padding.

See `docs/porting/pretext.md` in the repository for the complete port report.
The upstream MIT notice is included in `NOTICE.md`.
