# ferroui-pretext-media

Connects prepared text layout to FerroUI's configured font and shaping services.
It uses the framework's Unicode bidi algorithm, shapeable-character and font
fallback selection, platform `TextShaper`, and native `GlyphRun` objects. The
portable glyph output comes from actual shaping, including ligatures, combining
marks and right-to-left ordering.

## Prepared layout

Create the factory after configuring the application's FerroUI font and shaping
backends. The factory works with whichever backend the application installed;
this crate does not load a separate copy of a platform text library.

```rust
use ferroui_pretext::{PrepareOptions, PretextLayout};
use ferroui_pretext_media::FerroTextMeasurerFactory;

let factory = FerroTextMeasurerFactory::try_current()?;
let layout = factory.create_layout();
let prepared = layout.prepare_with_segments(
    "Hello 世界 — שלום",
    "16px sans-serif",
    PrepareOptions::default(),
)?;

// Changing the available width reuses the measured segments.
let narrow = PretextLayout::layout_with_lines(&prepared, 160.0, 22.0);
let wide = PretextLayout::layout_with_lines(&prepared, 320.0, 22.0);
```

`ferroui_pretext_media::create_layout()` is a shortcut when no explicit factory
handle or culture is needed. `FerroTextMeasurerFactory::with_culture` selects the
culture supplied to font fallback and the platform shaper.

## Native glyph drawing

The portable shaping contract exposes glyph positions, advances, font run
identities and UTF-16 cluster offsets. For drawing, keep the native result so the
actual selected font faces remain available:

```rust
use ferroui_base::Point;
use ferroui_pretext::PretextShapeOptions;

let font = factory.create_text_service("16px sans-serif")?;
let native = font.shape_native("Hello 世界 — שלום", PretextShapeOptions::default())?;

// `sink` implements FerroUI's ITextDrawingSink. Coordinates are the baseline
// origin in device-independent units; `foreground` is an optional brush.
native.draw(sink, foreground, Point::new(12.0, 30.0));
```

`FerroShapedText::glyph_runs()` also exposes the native runs for direct framework
integration. Runs are in visual order and carry their horizontal baseline
positions. `shaped_run()` exposes the corresponding portable data. Font run
names identify the actual resolved faces, including fallback fonts, rather than
only repeating the requested CSS-style font string.

Measurement uses the same shaping path but avoids building portable glyph
records and drawing resources. `FerroTextService::try_measure_text` reports
backend failures directly. The upstream-compatible measurement trait has no
error return, so it reports an unavailable backend as a non-finite value; core
preparation rejects that value.

## Lifetime and errors

Factories and services use the thread-local font-service scope in which they
were created. They return an error if the scope changes or required services are
absent. Construct a new factory after changing the application's font services.
Replacing a Pretext engine's configured backend preserves the shaper captured
by its existing prepared shaped text. Native glyph results retain their actual
font faces independently of the factory.

The adapter checks native source coordinates and metrics before publishing
portable glyph data. Cluster offsets are UTF-16 code units into the original
string, including for supplementary characters; glyph order can differ from
cluster order in right-to-left text. No unsafe code or platform FFI is introduced
by this crate.

## Example and tests

```sh
cargo run -p ferroui-pretext-media --example prepared_labels
cargo test -p ferroui-pretext-media
```

The console example installs FerroUI's deterministic headless text test services,
prepares one multilingual label, reflows it at two widths, and records native
glyph drawing operations. An application supplies its configured platform
services and drawing sink to the same adapter APIs.

Tests exercise font fallback, native font identities, bidi visual order, UTF-16
clusters, ligatures, combining marks, embedded NULs, drawing origins, backend
availability and prepared line materialization. Measurement is also checked
against FerroUI's text formatter with the same services. These are deterministic
integration checks; they do not assert rendered output from an operating-system
font installation.

See the [port report](https://github.com/wieslawsoltes/FerroUI/blob/main/docs/porting/pretext.md)
for source mapping and compatibility details.
