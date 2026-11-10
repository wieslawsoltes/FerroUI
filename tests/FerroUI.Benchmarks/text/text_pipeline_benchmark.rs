//! The text pipeline, stage by stage, on the texts of the gauntlet: what finding the break
//! opportunities, shaping, wrapping, wrapping again at a new width, measuring a text block and
//! making the glyph runs of the lines each cost, with the real shaper (HarfBuzz), font manager
//! and render interface (Skia). An addition: it locates where the time of text layout goes, so
//! that a change of algorithm is measured stage by stage (docs/porting/text-layout-performance.md).
//!
//! The corpora are generated, as the gauntlet's posts are:
//!
//! - `Card`: 500 card bodies of 22 to 38 words, the bodies of the gauntlet's cards;
//! - `Label`: 2,000 short labels (identifiers, prices, changes, counts), the gauntlet's tickers
//!   and chips;
//! - `Article`: 12 paragraphs of 400 to 700 words.
//!
//! The width sweep is the gauntlet's case: the width changes in every frame, so every wrapped
//! text is wrapped again; an operation lays out every text of the corpus at the next width of a
//! sweep from 180 to 320 in steps of 7.

use crate::harness::Registry;
use ferroui_base::media::text_formatting::unicode::LineBreakEnumerator;
use ferroui_base::media::text_formatting::{ITextDrawingSink, TextLayout, TextLayoutOptions, TextRunCache};
use ferroui_base::media::{Brushes, GlyphRun, IBrush, IPen, TextTrimming, TextWrapping, Typeface};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{Matrix, Point, Rect, Size};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::TextBlock;
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_skia::{FontManagerImpl, PlatformRenderInterface};
use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;

const WORDS: [&str; 46] = [
    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit", "sed", "do",
    "eiusmod", "tempor", "incididunt", "ut", "labore", "et", "dolore", "magna", "aliqua", "enim",
    "ad", "minim", "veniam", "quis", "nostrud", "exercitation", "ullamco", "laboris", "nisi",
    "aliquip", "ex", "ea", "commodo", "consequat", "duis", "aute", "irure", "in", "reprehenderit",
    "voluptate", "velit", "esse", "cillum", "fugiat", "nulla", "pariatur",
];

const FONT_SIZE: f64 = 14.0;
const WIDTH: f64 = 240.0;

/// The gauntlet's generator: 32-bit xorshift.
pub(super) struct Rng(u32);

impl Rng {
    pub(super) fn new(seed: u32) -> Self {
        Self((seed.wrapping_mul(0x9E37_79B9) ^ 0xA5A5_A5A5).max(1))
    }

    pub(super) fn below(&mut self, bound: u32) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x % bound
    }
}

pub(super) fn sentence(rng: &mut Rng, min: u32, max: u32) -> String {
    let count = min + rng.below(max - min + 1);
    let text = (0..count).map(|_| WORDS[rng.below(46) as usize]).collect::<Vec<_>>().join(" ");
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str() + ".",
        None => text,
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Corpus {
    Card,
    Label,
    Article,
}

impl Corpus {
    fn texts(self) -> Vec<String> {
        match self {
            Corpus::Card => (0..500).map(|index| sentence(&mut Rng::new(index + 1), 22, 38)).collect(),
            Corpus::Label => (0..2000u32)
                .map(|index| {
                    let mut rng = Rng::new(index + 7);
                    match index % 4 {
                        0 => format!("C{}.L{}.{}", rng.below(40), rng.below(9), rng.below(6)),
                        1 => format!("${},{:03}.{:02}", rng.below(90), rng.below(1000), rng.below(100)),
                        2 => format!("{}{}.{:02}%", if rng.below(2) == 0 { "+" } else { "-" }, rng.below(9), rng.below(100)),
                        _ => format!("{}.{}k", rng.below(20), rng.below(10)),
                    }
                })
                .collect(),
            Corpus::Article => (0..12)
                .map(|index| {
                    let mut rng = Rng::new(1000 + index);
                    let mut paragraph = String::new();
                    while paragraph.split(' ').count() < 400 + rng.below(300) as usize {
                        if !paragraph.is_empty() {
                            paragraph.push(' ');
                        }
                        paragraph += &sentence(&mut rng, 8, 20);
                    }
                    paragraph
                })
                .collect(),
        }
    }
}

/// The widths of the sweep, one an operation.
fn sweep_width(step: u32) -> f64 {
    180.0 + f64::from(step % 21) * 7.0
}

/// A sink that makes the glyph run of every run it is given draw, as a drawing context does
/// (the platform glyph run, its glyph positions and ink bounds), and draws nothing.
struct GlyphRunSink {
    runs: usize,
}

impl ITextDrawingSink for GlyphRunSink {
    fn draw_glyph_run(&mut self, _foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        black_box(glyph_run.platform_impl());
        self.runs += 1;
    }
    fn draw_rectangle(&mut self, _brush: Option<&Rc<dyn IBrush>>, _pen: Option<&Rc<dyn IPen>>, _rect: Rect) {}
    fn draw_line(&mut self, _pen: &Rc<dyn IPen>, _p1: Point, _p2: Point) {}
    fn push_transform(&mut self, _matrix: Matrix) {}
    fn pop_transform(&mut self) {}
}

pub struct TextPipelineBenchmark {
    texts: Vec<ReadOnlyMemory<u16>>,
    /// One cache of text runs a text, as a text block keeps one.
    caches: Vec<Rc<TextRunCache>>,
    blocks: Vec<ferroui_base::Ref<TextBlock>>,
    step: Cell<u32>,
    _app: UnitTestApplicationScope,
}

impl TextPipelineBenchmark {
    pub fn new(corpus: Corpus) -> Self {
        let services = TestServices::styled_window()
            .with_render_interface(Rc::new(PlatformRenderInterface::default()))
            .with_text_shaper_impl(Rc::new(HarfBuzzTextShaper::new()))
            .with_font_manager_impl(Rc::new(FontManagerImpl::new()));
        let app = UnitTestApplication::start(services);
        let strings = corpus.texts();
        let texts: Vec<_> = strings.iter().map(|text| ReadOnlyMemory::<u16>::from_str(text)).collect();
        let caches = texts.iter().map(|_| Rc::new(TextRunCache::new())).collect();
        let blocks = strings
            .iter()
            .map(|text| {
                let block = TextBlock::new();
                block.set_font_size(FONT_SIZE);
                block.set_text_wrapping(TextWrapping::Wrap);
                block.set_text(Some(text));
                block
            })
            .collect();
        let benchmark = Self { texts, caches, blocks, step: Cell::new(0), _app: app };
        // Shapes every text once: the warm benchmarks start from cached runs.
        benchmark.layout_all(WIDTH, true);
        benchmark
    }

    fn layout(&self, index: usize, width: f64, cached: bool) -> TextLayout {
        TextLayout::from_utf16(
            self.texts[index].clone(),
            Typeface::default_typeface(),
            TextLayoutOptions {
                font_size: FONT_SIZE,
                foreground: Some(Brushes::black()),
                max_width: width,
                text_trimming: Some(<dyn TextTrimming>::none()),
                text_wrapping: TextWrapping::Wrap,
                text_run_cache: cached.then(|| self.caches[index].clone()),
                ..TextLayoutOptions::default()
            },
        )
    }

    fn layout_all(&self, width: f64, cached: bool) -> usize {
        let mut lines = 0;
        for index in 0..self.texts.len() {
            let layout = self.layout(index, width, cached);
            lines += layout.text_lines().len();
            black_box(layout.height());
            layout.dispose();
        }
        lines
    }

    fn next_width(&self) -> f64 {
        let step = self.step.get();
        self.step.set(step.wrapping_add(1));
        sweep_width(step)
    }

    /// The UAX #14 break opportunities of every text, alone.
    pub fn break_opportunities(&self) -> usize {
        let mut count = 0;
        for text in &self.texts {
            let mut breaks = LineBreakEnumerator::new(text.span());
            while breaks.move_next().is_some() {
                count += 1;
            }
        }
        count
    }

    /// Every text laid out from nothing: shaping, breaking and the lines.
    pub fn layout_cold(&self) -> usize {
        self.layout_all(WIDTH, false)
    }

    /// Every text laid out at the width it had, its runs cached: the steady state.
    pub fn layout_same_width(&self) -> usize {
        self.layout_all(WIDTH, true)
    }

    /// Every text laid out at the next width of the sweep, its runs cached: the gauntlet's case.
    pub fn layout_rewrap_sweep(&self) -> usize {
        self.layout_all(self.next_width(), true)
    }

    /// Every text laid out at the next width and drawn into a sink that makes its glyph runs.
    pub fn layout_rewrap_and_glyph_runs(&self) -> usize {
        let width = self.next_width();
        let mut sink = GlyphRunSink { runs: 0 };
        for index in 0..self.texts.len() {
            let layout = self.layout(index, width, true);
            layout.draw(&mut sink, Point::new(0.0, 0.0));
            layout.dispose();
        }
        sink.runs
    }

    /// Every text block measured at the next width: the control's path, with its own caches.
    pub fn text_block_measure_sweep(&self) -> f64 {
        let width = self.next_width();
        let mut height = 0.0;
        for block in &self.blocks {
            block.measure(Size::new(width, f64::INFINITY));
            height += block.desired_size().height;
        }
        height
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "TextPipelineBenchmark").min_iteration_time(200);
    for corpus in [Corpus::Card, Corpus::Label, Corpus::Article] {
        let parameters = format!("Corpus={corpus:?}");
        class.benchmark("break_opportunities", parameters.clone(), move || TextPipelineBenchmark::new(corpus), |b| b.break_opportunities());
        class.benchmark("layout_cold", parameters.clone(), move || TextPipelineBenchmark::new(corpus), |b| b.layout_cold()).baseline();
        class.benchmark("layout_same_width", parameters.clone(), move || TextPipelineBenchmark::new(corpus), |b| b.layout_same_width());
        class.benchmark("layout_rewrap_sweep", parameters.clone(), move || TextPipelineBenchmark::new(corpus), |b| b.layout_rewrap_sweep());
        class.benchmark(
            "layout_rewrap_and_glyph_runs",
            parameters.clone(),
            move || TextPipelineBenchmark::new(corpus),
            |b| b.layout_rewrap_and_glyph_runs(),
        );
        class.benchmark(
            "text_block_measure_sweep",
            parameters.clone(),
            move || TextPipelineBenchmark::new(corpus),
            |b| b.text_block_measure_sweep(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "TextPipelineBenchmark");
    }

    /// `TextLayout::width_hold` with the real shaper and font (kerning, fractional advances): at
    /// every width a layout holds, in quarter-pixel steps, a new layout makes the same lines.
    #[test]
    fn width_holds_match_new_layouts_with_harfbuzz() {
        let benchmark = TextPipelineBenchmark::new(Corpus::Card);
        let lines = |layout: &TextLayout| -> Vec<(i32, i32)> {
            layout.text_lines().iter().map(|line| (line.first_text_source_index(), line.length())).collect()
        };
        let mut held = 0;
        for index in (0..benchmark.texts.len()).step_by(10) {
            for start in [150.0, 197.5, 240.0, 333.3] {
                let first = benchmark.layout(index, start, true);
                let expected = lines(&first);
                let Some((low, high)) = first.width_hold() else { continue };
                let mut width = (low - 1.0).max(1.0);
                while width < high.min(start + 200.0) + 1.0 {
                    if low <= width && width < high {
                        held += 1;
                        let fresh = benchmark.layout(index, width, true);
                        assert_eq!(expected, lines(&fresh), "text {index} created at {start}, held over {low}..{high}, other lines at {width}");
                        fresh.dispose();
                    }
                    width += 0.25;
                }
                first.dispose();
            }
        }
        assert!(held > 1_000, "only {held} widths were held");
    }
}
