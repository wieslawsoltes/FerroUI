//! A benchmark of one variant, used as a profiling target. It pins the most
//! expensive realistic case of [`HugeTextLayout`](super::huge_text_layout::HugeTextLayout)
//! (wrapping without trimming on the emoji block), so that a profiler
//! captures only this one run. Not part of the regular coverage: run it with
//! a filter.
//!
//! [`profile_text_layout`] is the profiling entry of the main function of
//! upstream: the same layout built for a fixed time without the harness.

use super::huge_text_layout::EMOJIS_TEXT;
use crate::harness::Registry;
use ferroui_base::media::text_formatting::{TextLayout, TextLayoutOptions, TextRunCache};
use ferroui_base::media::{Brushes, TextTrimming, TextWrapping, Typeface};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_skia::{FontManagerImpl, PlatformRenderInterface};
use std::rc::Rc;
use std::time::{Duration, Instant};

const EMOJIS: &str = EMOJIS_TEXT;

pub struct TextLayoutProfile {
    // Real consumers (a text block, for one) share a cache of text runs
    // across the layouts of the same string, so the cost of shaping
    // amortises. The benchmark mirrors that scenario; without the cache
    // shaping dominates and obscures changes of the wrapping loop.
    run_cache: Rc<TextRunCache>,
    /// The text in UTF-16, as the string of upstream is: a layout takes it
    /// without transcoding.
    emojis: ReadOnlyMemory<u16>,
    /// Disposed when the benchmark is dropped (the fields drop in the order
    /// of their declaration).
    _app: UnitTestApplicationScope,
}

impl TextLayoutProfile {
    pub fn new() -> Self {
        let run_cache = Rc::new(TextRunCache::new());

        let app = UnitTestApplication::start(TestServices::styled_window());

        Self { run_cache, emojis: ReadOnlyMemory::<u16>::from_str(EMOJIS), _app: app }
    }

    pub fn build_emojis_wrapped(&self) -> TextLayout {
        let layout = TextLayout::from_utf16(
            self.emojis.clone(),
            Typeface::default_typeface(),
            TextLayoutOptions {
                font_size: 12.0,
                foreground: Some(Brushes::black()),
                max_width: 120.0,
                text_trimming: Some(<dyn TextTrimming>::none()),
                text_wrapping: TextWrapping::WrapWithOverflow,
                text_run_cache: Some(self.run_cache.clone()),
                ..TextLayoutOptions::default()
            },
        );
        layout.dispose();
        layout
    }
}

/// The profiling entry of upstream: builds the layout of the emoji block
/// with a warm cache for eighteen seconds, without the harness, for a
/// sampling profiler to attach to, and prints how many layouts it built.
pub fn profile_text_layout() {
    // Bootstrap with the real Skia render interface (instead of the mock of
    // the styled window services). That makes the construction of a glyph
    // run, which builds the absolute position of every glyph and walks the
    // ink bounds, appear on the hot path, so we can see whether the prefix
    // sum of the render side is a real production cost. The mock platform
    // returns trivial values and hides this work.
    let services = TestServices::styled_window()
        .with_render_interface(Rc::new(PlatformRenderInterface::default()))
        .with_text_shaper_impl(Rc::new(HarfBuzzTextShaper::new()))
        .with_font_manager_impl(Rc::new(FontManagerImpl::new()));
    let _app = UnitTestApplication::start(services);

    // The text in UTF-16, as the string of upstream is.
    let text = ReadOnlyMemory::<u16>::from_str(EMOJIS_TEXT);

    // Real consumers (a text block, for one) share a cache of text runs
    // across the layouts of the same string, so the cost of shaping
    // amortises after the first build. Profiling without a cache makes the
    // shaper dominate the trace and hides the steady-state hot paths that
    // matter.
    let cache = Rc::new(TextRunCache::new());

    // Warm up the font loading and the trie data, and populate the cache so
    // the measurement loop runs with a warm cache as a real paint pass
    // would.
    for _ in 0..500 {
        build_one(&text, &cache).dispose();
    }

    // Steady-state measurement loop: about 18 s, which is well within the
    // 25 s a profiler is typically given to collect, so the entry returns
    // before the collector does.
    let sw = Instant::now();
    let mut count = 0_u32;
    while sw.elapsed() < Duration::from_secs(18) {
        build_one(&text, &cache).dispose();
        count += 1;
    }
    let elapsed = sw.elapsed();

    println!(
        "Built {count} layouts in {:.1}s ({:.3} ms/op).",
        elapsed.as_secs_f64(),
        elapsed.as_secs_f64() * 1000.0 / f64::from(count)
    );
}

fn build_one(text: &ReadOnlyMemory<u16>, cache: &Rc<TextRunCache>) -> TextLayout {
    TextLayout::from_utf16(
        text.clone(),
        Typeface::default_typeface(),
        TextLayoutOptions {
            font_size: 12.0,
            foreground: Some(Brushes::black()),
            max_width: 120.0,
            text_trimming: Some(<dyn TextTrimming>::none()),
            text_wrapping: TextWrapping::WrapWithOverflow,
            text_run_cache: Some(cache.clone()),
            ..TextLayoutOptions::default()
        },
    )
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "TextLayoutProfile").min_iteration_time(150);
    class.benchmark("build_emojis_wrapped", "", TextLayoutProfile::new, |b| b.build_emojis_wrapped());
}

#[cfg(test)]
mod tests {
    #[test]
    fn text_layout_profile() {
        crate::harness::smoke_class(super::register, "TextLayoutProfile");
    }
}
