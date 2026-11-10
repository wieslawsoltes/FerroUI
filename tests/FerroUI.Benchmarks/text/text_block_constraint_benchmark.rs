//! A text block measured at one width and arranged at another: the gauntlet's case at its small
//! tiers. A parent hands a text block the width it has left, which is not rounded, and the block
//! is arranged at its layout-rounded width, so it sees two constraints in every layout pass. An
//! addition (docs/porting/desktop-performance.md section 9.27), with the real shaper (HarfBuzz),
//! font manager and render interface (Skia); the texts are generated as the gauntlet's are:
//!
//! - `Paragraph`: 500 bodies of 6 to 12 words that wrap, with a character ellipsis and a limit of
//!   six lines that few of them reach (the gauntlet's titles and card bodies);
//! - `Subtitle`: 500 lines of four inlines (`by `, an author in bold, a separator, a tag) that do
//!   not wrap, with a character ellipsis and a limit of one line (the gauntlet's post headers).
//!
//! An operation measures every text block at the unrounded width and arranges it at the rounded
//! one. A text block that keeps its layout over the two lays nothing out.

use super::text_pipeline_benchmark::{sentence, Rng};
use crate::harness::Registry;
use ferroui_base::media::{FontWeight, TextTrimming, TextWrapping};
use ferroui_base::{Rect, Size};
use ferroui_controls::documents::{InlineCollection, Run};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::TextBlock;
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_skia::{FontManagerImpl, PlatformRenderInterface};
use std::rc::Rc;

const FONT_SIZE: f64 = 12.0;
/// The width a parent has left for the text, and the layout-rounded width of the text block.
const MEASURED_WIDTH: f64 = 179.763;
const ARRANGED_WIDTH: f64 = 180.0;

const AUTHORS: [&str; 6] = ["Ada Lovelace", "Alan Turing", "Grace Hopper", "Edsger Dijkstra", "Barbara Liskov", "Ken Thompson"];
const TAGS: [&str; 5] = ["#rust", "#layout", "#text", "#gpu", "#ui"];

#[derive(Clone, Copy, Debug)]
pub enum Texts {
    Paragraph,
    Subtitle,
}

fn make_blocks(texts: Texts) -> Vec<ferroui_base::Ref<TextBlock>> {
    (0..500u32)
        .map(|index| {
            let block = TextBlock::new();
            block.set_font_size(FONT_SIZE);
            block.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
            match texts {
                Texts::Paragraph => {
                    block.set_text_wrapping(TextWrapping::Wrap);
                    block.set_max_lines(6);
                    block.set_text(Some(&sentence(&mut Rng::new(index + 1), 6, 12)));
                }
                Texts::Subtitle => {
                    block.set_max_lines(1);
                    let author = Run::with_text(Some(AUTHORS[index as usize % AUTHORS.len()]));
                    author.set_font_weight(FontWeight::Bold);
                    let inlines = InlineCollection::new();
                    inlines.add(Run::with_text(Some("by ")));
                    inlines.add(&author);
                    inlines.add(Run::with_text(Some(" \u{b7} ")));
                    inlines.add(Run::with_text(Some(TAGS[index as usize % TAGS.len()])));
                    block.set_inlines(Some(inlines));
                }
            }
            block
        })
        .collect()
}

pub struct TextBlockConstraintBenchmark {
    blocks: Vec<ferroui_base::Ref<TextBlock>>,
    _app: UnitTestApplicationScope,
}

impl TextBlockConstraintBenchmark {
    pub fn new(texts: Texts) -> Self {
        let services = TestServices::styled_window()
            .with_render_interface(Rc::new(PlatformRenderInterface::default()))
            .with_text_shaper_impl(Rc::new(HarfBuzzTextShaper::new()))
            .with_font_manager_impl(Rc::new(FontManagerImpl::new()));
        let app = UnitTestApplication::start(services);
        let blocks = make_blocks(texts);
        let benchmark = Self { blocks, _app: app };
        // Shapes every text once: the benchmark starts from cached runs.
        benchmark.measure_then_arrange_rounded();
        benchmark
    }

    /// Every text block measured at the unrounded width, then arranged at the rounded one.
    pub fn measure_then_arrange_rounded(&self) -> f64 {
        let mut height = 0.0;
        for block in &self.blocks {
            block.measure(Size::new(MEASURED_WIDTH, f64::INFINITY));
            let desired = block.desired_size().height;
            block.arrange(Rect::new(0.0, 0.0, ARRANGED_WIDTH, desired));
            height += desired;
        }
        height
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "TextBlockConstraintBenchmark").min_iteration_time(200);
    for texts in [Texts::Paragraph, Texts::Subtitle] {
        class.benchmark(
            "measure_then_arrange_rounded",
            format!("Texts={texts:?}"),
            move || TextBlockConstraintBenchmark::new(texts),
            |b| b.measure_then_arrange_rounded(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "TextBlockConstraintBenchmark");
    }

    fn line_ranges(block: &TextBlock) -> Vec<(i32, i32)> {
        block.text_layout().text_lines().iter().map(|line| (line.first_text_source_index(), line.length())).collect()
    }

    /// With the real shaper and font: a text block keeps its layout over the measured and the
    /// arranged width unless a line was trimmed or left out, and the kept layout has the lines
    /// of a layout made at the arranged width.
    #[test]
    fn layouts_are_kept_over_the_two_widths_and_have_the_lines_of_new_ones() {
        for texts in [Texts::Paragraph, Texts::Subtitle] {
            let benchmark = TextBlockConstraintBenchmark::new(texts);
            // Blocks that only ever see the arranged width.
            let fresh = make_blocks(texts);
            let before: Vec<_> = benchmark.blocks.iter().map(|block| block.text_layout()).collect();
            benchmark.measure_then_arrange_rounded();
            let mut kept = 0;
            for ((block, before), fresh) in benchmark.blocks.iter().zip(&before).zip(&fresh) {
                if Rc::ptr_eq(before, &block.text_layout()) {
                    // That a kept layout has no trimmed or left out line is checked with the
                    // layout itself (`text_layout_width_hold_tests`): this crate also builds
                    // against the base of a gate run, which may be older than those accessors.
                    kept += 1;
                }
                fresh.measure(Size::new(ARRANGED_WIDTH, f64::INFINITY));
                assert_eq!(line_ranges(fresh), line_ranges(block), "{texts:?}");
                assert_eq!(fresh.text_layout().height(), block.text_layout().height(), "{texts:?}");
            }
            assert!(kept >= 450, "{texts:?}: only {kept} of 500 layouts were kept");
        }
    }
}
