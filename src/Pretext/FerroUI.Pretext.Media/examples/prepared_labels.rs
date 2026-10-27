//! Prepared layout and native glyph drawing through FerroUI's text services.
//!
//! This console example installs deterministic test fonts so it runs without a
//! display server. In an application, configure the regular FerroUI platform
//! and render backends before creating the Pretext factory, then pass the real
//! text drawing sink to `native.draw`.

use ferroui_base::media::text_formatting::testing::{RecordingDrawingSink, TextTestScope};
use ferroui_base::Point;
use ferroui_pretext::{PrepareOptions, PretextError, PretextLayout, PretextShapeOptions};
use ferroui_pretext_media::FerroTextMeasurerFactory;

fn main() -> Result<(), PretextError> {
    let _services = TextTestScope::new();
    let factory = FerroTextMeasurerFactory::try_current()?;
    let layout = factory.create_layout();
    let font = "16px Test Sans";
    let service = factory.create_text_service(font)?;
    let prepared = layout.prepare_with_segments(
        "Hello 世界 😀 — שלום. Prepare once, reflow at different widths.",
        font,
        PrepareOptions::default(),
    )?;

    // Width changes reuse the measured segments. Only lines being drawn need
    // text materialization and native glyph runs.
    for max_width in [120.0, 240.0] {
        let mut ranges = Vec::new();
        PretextLayout::walk_line_ranges(&prepared, max_width, |line| ranges.push(line));
        let mut sink = RecordingDrawingSink::new();
        for (line_index, range) in ranges.iter().enumerate() {
            let line = PretextLayout::materialize_line_range(&prepared, range);
            let native = service.shape_native(&line.text, PretextShapeOptions::default())?;
            native.draw(
                &mut sink,
                None,
                Point::new(12.0, 20.0 + line_index as f64 * 22.0),
            );
            println!(
                "{max_width:>3}px line {line_index}: {:?}, {:.1}px, {} glyphs in {} font runs",
                line.text,
                line.width,
                native.shaped_run().glyphs().len(),
                native.shaped_run().font_runs().len(),
            );
        }
        println!("{} recorded native glyph-run draws", sink.calls().len());
    }
    Ok(())
}
