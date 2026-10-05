//! Glyph metrics tables (`hmtx`, `vmtx`).

mod horizontal_glyph_metric;
mod horizontal_metrics_table;
mod vertical_glyph_metric;
mod vertical_metrics_table;

pub use horizontal_glyph_metric::HorizontalGlyphMetric;
pub use horizontal_metrics_table::HorizontalMetricsTable;
pub use vertical_glyph_metric::VerticalGlyphMetric;
pub use vertical_metrics_table::VerticalMetricsTable;
