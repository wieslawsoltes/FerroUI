//! Text blocks whose short texts change in every frame: the gauntlet's values (counters, prices,
//! percentages), which are 744 of the 796 layouts one of its frames makes. An addition
//! (docs/porting/desktop-performance.md section 9.29), with the real shaper (HarfBuzz), font
//! manager and render interface (Skia).
//!
//! 800 text blocks of one font size and eight shared brushes show values of two to six
//! characters. An operation is a frame: every block gets another value and is measured.
//!
//! - `Repeating`: the values come from 12,000, so a value was shown before and, as in the
//!   gauntlet, a while ago: a layout kept by its content is found for somewhat under half of
//!   them;
//! - `New`: no value was shown before (a counter that never repeats): what the text block costs
//!   when nothing that is kept can be found.

use super::text_pipeline_benchmark::Rng;
use crate::harness::Registry;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::Size;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::TextBlock;
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_skia::{FontManagerImpl, PlatformRenderInterface};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const BLOCKS: u32 = 800;
const VALUES: u32 = 12_000;
const FONT_SIZE: f64 = 12.0;
/// The width a cell has for its value.
const WIDTH: f64 = 96.0;

#[derive(Clone, Copy, Debug)]
pub enum Values {
    Repeating,
    New,
}

pub struct TextBlockChangingTextBenchmark {
    blocks: Vec<ferroui_base::Ref<TextBlock>>,
    values: Values,
    /// The values of `Repeating`, made once: an operation does not format.
    pool: Vec<String>,
    rng: RefCell<Rng>,
    next: Cell<u64>,
    _app: UnitTestApplicationScope,
}

impl TextBlockChangingTextBenchmark {
    pub fn new(values: Values) -> Self {
        let services = TestServices::styled_window()
            .with_render_interface(Rc::new(PlatformRenderInterface::default()))
            .with_text_shaper_impl(Rc::new(HarfBuzzTextShaper::new()))
            .with_font_manager_impl(Rc::new(FontManagerImpl::new()));
        let app = UnitTestApplication::start(services);

        // The gauntlet's swatches: a few brushes shared by all the blocks.
        let brushes: Vec<Rc<dyn IBrush>> = (0..8u8)
            .map(|index| -> Rc<dyn IBrush> {
                Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(40 + index * 20, 90, 200 - index * 20)))
            })
            .collect();
        let blocks = (0..BLOCKS)
            .map(|index| {
                let block = TextBlock::new();
                block.set_font_size(FONT_SIZE);
                block.set_foreground(Some(brushes[index as usize % brushes.len()].clone()));
                block
            })
            .collect();
        // Two to six characters: "17", "4.2", "12.34", "99.9%", "1,204".
        let pool = (0..VALUES)
            .map(|index| match index % 5 {
                0 => format!("{}", index / 5 % 100),
                1 => format!("{}.{}", index / 5 % 10, index / 50 % 10),
                2 => format!("{}.{:02}", index / 5 % 100, index / 500 % 100),
                3 => format!("{}.{}%", index / 5 % 100, index / 500 % 10),
                _ => format!("{},{:03}", 1 + index / 5 % 9, index / 45 % 1000),
            })
            .collect();

        let benchmark = Self { blocks, values, pool, rng: RefCell::new(Rng::new(7)), next: Cell::new(0), _app: app };
        // Frames before the measured ones: the fonts are loaded and what is kept is in use.
        for _ in 0..8 {
            benchmark.change_and_measure();
        }
        benchmark
    }

    /// A frame: every text block gets another value and is measured.
    pub fn change_and_measure(&self) -> f64 {
        let mut rng = self.rng.borrow_mut();
        let mut width = 0.0;
        let mut text = String::new();
        for block in &self.blocks {
            match self.values {
                Values::Repeating => block.set_text(Some(&self.pool[rng.below(VALUES) as usize])),
                Values::New => {
                    use std::fmt::Write;
                    let next = self.next.get();
                    self.next.set(next + 1);
                    text.clear();
                    let _ = write!(text, "{}", next % 1_000_000);
                    block.set_text(Some(&text));
                }
            }
            block.measure(Size::new(WIDTH, f64::INFINITY));
            width += block.desired_size().width;
        }
        width
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "TextBlockChangingTextBenchmark").min_iteration_time(200);
    for values in [Values::Repeating, Values::New] {
        class.benchmark(
            "change_and_measure",
            format!("Values={values:?}"),
            move || TextBlockChangingTextBenchmark::new(values),
            |b| b.change_and_measure(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "TextBlockChangingTextBenchmark");
    }
}
