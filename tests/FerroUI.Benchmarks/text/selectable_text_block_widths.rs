//! Selectable text blocks laid out at two widths in turn, as a parent that hands them the width
//! it has left and then arranges them at another does: each width wraps the text otherwise, so
//! each is a new layout (step 4 of docs/porting/text-layout-performance.md). A layout that keeps
//! the shaped runs of the one before only wraps them; one without lays the text out from nothing.
//! With the real shaper (HarfBuzz), font manager and render interface (Skia).

use super::text_pipeline_benchmark::{sentence, Rng};
use crate::harness::Registry;
use ferroui_base::media::TextWrapping;
use ferroui_base::Size;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::SelectableTextBlock;
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_skia::{FontManagerImpl, PlatformRenderInterface};
use std::rc::Rc;

const BLOCKS: u32 = 500;
const WIDTHS: [f64; 2] = [180.0, 140.0];

pub struct SelectableTextBlockWidths {
    blocks: Vec<ferroui_base::Ref<SelectableTextBlock>>,
    next: usize,
    _app: UnitTestApplicationScope,
}

impl SelectableTextBlockWidths {
    fn new() -> Self {
        let services = TestServices::styled_window()
            .with_render_interface(Rc::new(PlatformRenderInterface::default()))
            .with_text_shaper_impl(Rc::new(HarfBuzzTextShaper::new()))
            .with_font_manager_impl(Rc::new(FontManagerImpl::new()));
        let app = UnitTestApplication::start(services);
        let blocks = (0..BLOCKS)
            .map(|index| {
                let block = SelectableTextBlock::new();
                block.set_text_wrapping(TextWrapping::Wrap);
                block.set_text(Some(&sentence(&mut Rng::new(index + 1), 10, 18)));
                block
            })
            .collect();
        let mut benchmark = Self { blocks, next: 0, _app: app };
        benchmark.measure_at_the_other_width();
        benchmark
    }

    /// Every block measured at the width it was not measured at last.
    pub fn measure_at_the_other_width(&mut self) -> f64 {
        let width = WIDTHS[self.next];
        self.next = 1 - self.next;
        let mut height = 0.0;
        for block in &self.blocks {
            block.measure(Size::new(width, f64::INFINITY));
            height += block.desired_size().height;
        }
        height
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("text", "SelectableTextBlockWidths").min_iteration_time(200);
    class.benchmark(
        "measure_at_the_other_width",
        format!("Blocks={BLOCKS}"),
        SelectableTextBlockWidths::new,
        |b| b.measure_at_the_other_width(),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "SelectableTextBlockWidths");
    }
}
