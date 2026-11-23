//! Not from upstream: what the Skia backend pays to create a glyph run, the step of every text line
//! that is formatted (`TextLineImpl::create_line_metrics` asks the render interface for the glyph
//! run of each shaped run). The run measures the ink bounds of its glyphs with the Skia font of its
//! size, which takes the strike cache of Skia, a cache of the whole process under one lock that the
//! threads drawing frames take for every text blob they draw.
//!
//! - `same_size`: runs of one typeface at one size, as most text of an application is.
//! - `eight_sizes`: the same runs at eight sizes in turn.
//! - `forty_sizes`: at forty sizes in turn, more sizes than a typeface keeps, as a text whose size
//!   is animated would be.

use crate::harness::Registry;
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{GlyphTypeface, Typeface};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::Point;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_skia::{FontManagerImpl, PlatformRenderInterface};
use std::rc::Rc;

/// The runs a benchmark iteration creates: a line of a few words.
const RUNS: usize = 8;

pub struct GlyphRunCreationBenchmark {
    render_interface: PlatformRenderInterface,
    glyph_typeface: Rc<GlyphTypeface>,
    runs: Vec<Vec<GlyphInfo>>,
    sizes: Vec<f64>,
    next_size: usize,
    /// Declared last: the application ends after the typeface is released.
    _app: UnitTestApplicationScope,
}

impl GlyphRunCreationBenchmark {
    pub fn new(glyph_count: usize, size_count: usize) -> Self {
        let app = UnitTestApplication::start(
            TestServices::mock_platform_render_interface()
                .with_render_interface(Rc::new(PlatformRenderInterface::default()))
                .with_font_manager_impl(Rc::new(FontManagerImpl::new())),
        );
        let glyph_typeface = Typeface::default().glyph_typeface();
        let map = glyph_typeface.character_to_glyph_map();
        let pool: Vec<u16> =
            ('!'..='~').filter(|c| map.contains_glyph(*c as i32)).map(|c| map.get_glyph(c as i32)).collect();
        // Every run its own glyphs from the printable range, as the words of a line are.
        let runs = (0..RUNS)
            .map(|run| {
                (0..glyph_count)
                    .map(|i| GlyphInfo::new(pool[(run * 7 + i * 3) % pool.len()], i as i32, 9.0))
                    .collect()
            })
            .collect();
        let sizes = (0..size_count).map(|i| 12.0 + i as f64 * 0.5).collect();
        Self { render_interface: PlatformRenderInterface::default(), glyph_typeface, runs, sizes, next_size: 0, _app: app }
    }

    /// Creates the glyph runs of one line at the next size of the rotation.
    pub fn create_runs(&mut self) {
        let size = self.sizes[self.next_size];
        self.next_size = (self.next_size + 1) % self.sizes.len();
        for glyphs in &self.runs {
            let run = self.render_interface.create_glyph_run(&self.glyph_typeface, size, glyphs, Point::new(0.0, 12.0));
            std::hint::black_box(run);
        }
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "GlyphRunCreationBenchmark");
    for glyph_count in [6, 24] {
        let parameters = format!("GlyphCount={glyph_count}");
        class.benchmark(
            "same_size",
            parameters.clone(),
            move || GlyphRunCreationBenchmark::new(glyph_count, 1),
            |b| b.create_runs(),
        );
        class.benchmark(
            "eight_sizes",
            parameters.clone(),
            move || GlyphRunCreationBenchmark::new(glyph_count, 8),
            |b| b.create_runs(),
        );
        class.benchmark(
            "forty_sizes",
            parameters,
            move || GlyphRunCreationBenchmark::new(glyph_count, 40),
            |b| b.create_runs(),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn glyph_run_creation_benchmark() {
        crate::harness::smoke_class(super::register, "GlyphRunCreationBenchmark");
    }
}
