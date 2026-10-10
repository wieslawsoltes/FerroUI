//! The hot paths of the cluster cache of a shaped buffer: the total glyph
//! advance, the leading characters that fit a width, and a chain of splits
//! that share the cache. The text is random ASCII, so the buffers are in the
//! simple mode of the cache (one character per cluster).

use super::random::Random;
use crate::harness::Registry;
use ferroui_base::media::text_formatting::{ShapedBuffer, TextShaper, TextShaperOptions};
use ferroui_base::media::Typeface;
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

pub struct ShapedBufferOps {
    options: TextShaperOptions,
    /// The text in UTF-16, as the string of upstream is: the shaper takes it
    /// without transcoding.
    text: ReadOnlyMemory<u16>,
    primed: Rc<ShapedBuffer>,
    /// Disposed when the benchmark is dropped, after the buffer (the fields
    /// drop in the order of their declaration).
    _app: UnitTestApplicationScope,
}

impl ShapedBufferOps {
    pub fn new(glyph_count: i32) -> Self {
        // Upstream starts the application with the services of tests that
        // draw through the platform render interface, which there have a
        // render interface, a font backend and a text shaper of tests. That
        // preset has none of the three here, so they are taken from the
        // services of a styled window, which has the ones of tests.
        let styled_window = TestServices::styled_window();
        let app = UnitTestApplication::start(TestServices {
            render_interface: styled_window.render_interface,
            font_manager_impl: styled_window.font_manager_impl,
            text_shaper_impl: styled_window.text_shaper_impl,
            ..TestServices::mock_platform_render_interface()
        });
        let options = TextShaperOptions::new(Typeface::default_typeface().glyph_typeface());

        // ASCII text: the simple mode fast path (one glyph == one cluster == one char).
        let mut rng = Random::new(glyph_count as u64);
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 ";
        let text: Vec<u16> = (0..glyph_count)
            .map(|_| u16::from(ALPHABET[rng.next_max(ALPHABET.len() as i32) as usize]))
            .collect();
        let text = ReadOnlyMemory::from_vec(text);

        let primed = TextShaper::current().shape_text(&text, &options);
        // Prime the cluster cache once so the per-call benchmarks measure only the lookup cost.
        let _ = primed.total_glyph_advance();

        Self { options, text, primed, _app: app }
    }

    /// Cost of shaping + building the cluster cache from scratch. Captures both
    /// the call of the shaper and the allocation and initialisation of the
    /// prefix sums.
    pub fn shape_and_prime(&self) -> f64 {
        let buffer = TextShaper::current().shape_text(&self.text, &self.options);
        let total = buffer.total_glyph_advance();
        buffer.dispose();
        total
    }

    /// Repeated `total_glyph_advance` on a primed buffer.
    /// Should be ~O(1) per call regardless of glyph count.
    pub fn total_advance_cached(&self) -> f64 {
        let mut sum = 0.0_f64;
        for _ in 0..64 {
            sum += self.primed.total_glyph_advance();
        }
        sum
    }

    /// Repeated `find_leading_char_count_within_width` targeting
    /// half the buffer's total width. Exercises the binary search across the
    /// prefix table.
    pub fn measure_fit_cached(&self) -> i32 {
        let half_width = self.primed.total_glyph_advance() * 0.5;
        let mut sum = 0_i32;
        for _ in 0..64 {
            sum += self.primed.find_leading_char_count_within_width(half_width);
        }
        sum
    }

    /// Splits the primed buffer at three positions and queries the resulting
    /// children: the workload the shared cluster cache is designed for. Each
    /// child should reuse the parent cache in O(1)/O(log) instead of rebuilding.
    pub fn split_chain(&self) -> f64 {
        let quarter = (self.primed.text().len() / 4) as i32;

        let halves = self.primed.split(quarter * 2);
        let first = halves.first.expect("the split has a first half");
        let second = halves.second.expect("the split has a second half");
        let first_split = first.split(quarter);
        let second_split = second.split(quarter);

        let first_first = first_split.first;
        let first_second = first_split.second;
        let second_first = second_split.first;
        let second_second = second_split.second;

        let total = first_first.as_ref().map_or(0.0, |buffer| buffer.total_glyph_advance())
            + first_second.as_ref().map_or(0.0, |buffer| buffer.total_glyph_advance())
            + second_first.as_ref().map_or(0.0, |buffer| buffer.total_glyph_advance())
            + second_second.as_ref().map_or(0.0, |buffer| buffer.total_glyph_advance());

        if let Some(buffer) = &first_first {
            buffer.dispose();
        }
        if let Some(buffer) = &first_second {
            buffer.dispose();
        }
        if let Some(buffer) = &second_first {
            buffer.dispose();
        }
        if let Some(buffer) = &second_second {
            buffer.dispose();
        }
        first.dispose();
        second.dispose();

        total
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "ShapedBufferOps").min_iteration_time(150);
    for glyph_count in [8, 32, 128, 512, 2048] {
        let parameters = format!("GlyphCount={glyph_count}");
        class.benchmark(
            "shape_and_prime",
            parameters.clone(),
            move || ShapedBufferOps::new(glyph_count),
            |b| b.shape_and_prime(),
        );
        class.benchmark(
            "total_advance_cached",
            parameters.clone(),
            move || ShapedBufferOps::new(glyph_count),
            |b| b.total_advance_cached(),
        );
        class.benchmark(
            "measure_fit_cached",
            parameters.clone(),
            move || ShapedBufferOps::new(glyph_count),
            |b| b.measure_fit_cached(),
        );
        class.benchmark("split_chain", parameters, move || ShapedBufferOps::new(glyph_count), |b| b.split_chain());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shaped_buffer_ops() {
        crate::harness::smoke_class(super::register, "ShapedBufferOps");
    }
}
