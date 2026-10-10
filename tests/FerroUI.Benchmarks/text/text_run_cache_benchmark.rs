//! Laying a text out again and again, with and without a cache of its
//! shaped runs.

use crate::harness::Registry;
use ferroui_base::media::text_formatting::{TextLayout, TextLayoutOptions, TextRunCache};
use ferroui_base::media::{Brushes, TextWrapping, Typeface};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

const SHORT_TEXT: &str = "The quick brown fox jumps over the lazy dog.";

const LONG_TEXT: &str = concat!(
    "Though, the objectives of the development of the prominent landmarks can be neglected in most cases, ",
    "it should be realized that after the completion of the strategic decision gives rise to ",
    "The Expertise of Regular Program. A number of key issues arise from the belief that the explicit ",
    "examination of strategic management should correlate with the conceptual design. ",
    "By all means, the unification of the reliably developed techniques indicates the importance of ",
    "the ultimate advantage of episodic skill over alternate practices.",
);

pub struct TextRunCacheBenchmark {
    iterations: i32,
    /// The texts in UTF-16, as the strings of upstream are: a layout takes
    /// them without transcoding.
    short_text: ReadOnlyMemory<u16>,
    long_text: ReadOnlyMemory<u16>,
    /// Disposed when the benchmark is dropped (the fields drop in the order
    /// of their declaration).
    _app: UnitTestApplicationScope,
}

impl TextRunCacheBenchmark {
    pub fn new(iterations: i32) -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());

        Self {
            iterations,
            short_text: ReadOnlyMemory::<u16>::from_str(SHORT_TEXT),
            long_text: ReadOnlyMemory::<u16>::from_str(LONG_TEXT),
            _app: app,
        }
    }

    /// The layout every benchmark of the class builds: the default typeface
    /// at 12, a black foreground, and the width, wrapping and cache given.
    fn layout(
        text: &ReadOnlyMemory<u16>,
        max_width: f64,
        text_wrapping: TextWrapping,
        text_run_cache: Option<&Rc<TextRunCache>>,
    ) -> TextLayout {
        TextLayout::from_utf16(
            text.clone(),
            Typeface::default_typeface(),
            TextLayoutOptions {
                font_size: 12.0,
                foreground: Some(Brushes::black()),
                max_width,
                text_wrapping,
                text_run_cache: text_run_cache.cloned(),
                ..TextLayoutOptions::default()
            },
        )
    }

    pub fn layout_without_cache_short(&self) {
        for _ in 0..self.iterations {
            let layout = Self::layout(&self.short_text, 200.0, TextWrapping::WrapWithOverflow, None);
            layout.dispose();
        }
    }

    pub fn layout_with_cache_short(&self) {
        let cache = Rc::new(TextRunCache::new());

        for _ in 0..self.iterations {
            let layout = Self::layout(&self.short_text, 200.0, TextWrapping::WrapWithOverflow, Some(&cache));
            layout.dispose();
        }

        cache.dispose();
    }

    pub fn layout_without_cache_long(&self) {
        for _ in 0..self.iterations {
            let layout = Self::layout(&self.long_text, 300.0, TextWrapping::WrapWithOverflow, None);
            layout.dispose();
        }
    }

    pub fn layout_with_cache_long(&self) {
        let cache = Rc::new(TextRunCache::new());

        for _ in 0..self.iterations {
            let layout = Self::layout(&self.long_text, 300.0, TextWrapping::WrapWithOverflow, Some(&cache));
            layout.dispose();
        }

        cache.dispose();
    }

    pub fn layout_without_cache_varying_width(&self) {
        for i in 0..self.iterations {
            let width = 200 + i * 10;

            let layout = Self::layout(&self.long_text, f64::from(width), TextWrapping::WrapWithOverflow, None);
            layout.dispose();
        }
    }

    pub fn layout_with_cache_varying_width(&self) {
        let cache = Rc::new(TextRunCache::new());

        for i in 0..self.iterations {
            let width = 200 + i * 10;

            let layout =
                Self::layout(&self.long_text, f64::from(width), TextWrapping::WrapWithOverflow, Some(&cache));
            layout.dispose();
        }

        cache.dispose();
    }

    /// Benchmarks the single-entry fast path: a simple single-paragraph text
    /// that results in only one cache entry (the common case for a text
    /// block).
    pub fn layout_with_cache_single_entry_short(&self) {
        let cache = Rc::new(TextRunCache::new());

        for _ in 0..self.iterations {
            // NoWrap + single paragraph = single cache entry at index 0.
            let layout = Self::layout(&self.short_text, f64::INFINITY, TextWrapping::NoWrap, Some(&cache));
            layout.dispose();
        }

        cache.dispose();
    }

    /// Benchmarks the single-entry fast path with invalidate/re-populate cycle,
    /// verifying that the inline store is reused without dictionary allocation.
    pub fn layout_with_cache_single_entry_invalidate_repopulate(&self) {
        let cache = Rc::new(TextRunCache::new());

        for _ in 0..self.iterations {
            cache.invalidate();

            let layout = Self::layout(&self.short_text, f64::INFINITY, TextWrapping::NoWrap, Some(&cache));
            layout.dispose();
        }

        cache.dispose();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "TextRunCacheBenchmark").min_iteration_time(150);
    for iterations in [5, 20] {
        let parameters = format!("Iterations={iterations}");
        class
            .benchmark(
                "layout_without_cache_short",
                parameters.clone(),
                move || TextRunCacheBenchmark::new(iterations),
                |b| b.layout_without_cache_short(),
            )
            .baseline();
        class.benchmark(
            "layout_with_cache_short",
            parameters.clone(),
            move || TextRunCacheBenchmark::new(iterations),
            |b| b.layout_with_cache_short(),
        );
        class.benchmark(
            "layout_without_cache_long",
            parameters.clone(),
            move || TextRunCacheBenchmark::new(iterations),
            |b| b.layout_without_cache_long(),
        );
        class.benchmark(
            "layout_with_cache_long",
            parameters.clone(),
            move || TextRunCacheBenchmark::new(iterations),
            |b| b.layout_with_cache_long(),
        );
        class.benchmark(
            "layout_without_cache_varying_width",
            parameters.clone(),
            move || TextRunCacheBenchmark::new(iterations),
            |b| b.layout_without_cache_varying_width(),
        );
        class.benchmark(
            "layout_with_cache_varying_width",
            parameters.clone(),
            move || TextRunCacheBenchmark::new(iterations),
            |b| b.layout_with_cache_varying_width(),
        );
        class.benchmark(
            "layout_with_cache_single_entry_short",
            parameters.clone(),
            move || TextRunCacheBenchmark::new(iterations),
            |b| b.layout_with_cache_single_entry_short(),
        );
        class.benchmark(
            "layout_with_cache_single_entry_invalidate_repopulate",
            parameters,
            move || TextRunCacheBenchmark::new(iterations),
            |b| b.layout_with_cache_single_entry_invalidate_repopulate(),
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn text_run_cache_benchmark() {
        crate::harness::smoke_class(super::register, "TextRunCacheBenchmark");
    }
}
