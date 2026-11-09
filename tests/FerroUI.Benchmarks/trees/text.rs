//! The text of many short labels, as tiles, tickers and counters have it (sections 9.4, 9.7, 9.8,
//! 9.12, 9.16).
//!
//! - `create_and_measure`: text blocks of five digits created and measured;
//! - `change_and_lay_out`: the text of every label of an attached stack changed to the next number
//!   and a layout pass run;
//! - `narrow_and_lay_out`: wrapping paragraphs of an attached panel laid out at a width that
//!   alternates between two close values, within the holds of their layouts.

use super::fixture::Fixture;
use crate::harness::Registry;
use ferroui_base::media::TextWrapping;
use ferroui_base::{Ref, Size};
use ferroui_controls::{Control, StackPanel, TextBlock, WrapPanel};
use std::cell::Cell;

/// The labels of a benchmark.
pub const LABELS: usize = 2_000;

pub struct Text {
    fixture: Fixture,
    labels: Vec<Ref<TextBlock>>,
    panel: Option<Ref<WrapPanel>>,
    frame: Cell<usize>,
}

impl Text {
    fn new() -> Self {
        Self { fixture: Fixture::new(), labels: Vec::new(), panel: None, frame: Cell::new(0) }
    }

    fn create_and_measure(&mut self) -> usize {
        let mut count = 0;
        for index in 0..LABELS {
            let label = TextBlock::new();
            label.set_font_size(12.0);
            label.set_text(Some(&format!("{index:05}")));
            label.measure(Size::new(f64::INFINITY, f64::INFINITY));
            count += label.desired_size().width as usize;
        }
        count
    }

    fn with_stack() -> Self {
        let mut this = Self::new();
        let stack = StackPanel::new();
        for index in 0..LABELS {
            let label = TextBlock::new();
            label.set_font_size(12.0);
            label.set_text(Some(&format!("{index:05}")));
            stack.children().add(&label);
            this.labels.push(label);
        }
        this.fixture.show(Some(&stack.upcast::<Control>()));
        this.fixture.lay_out();
        this
    }

    fn change_and_lay_out(&mut self) {
        let frame = self.frame.get() + 1;
        self.frame.set(frame);
        for (index, label) in self.labels.iter().enumerate() {
            label.set_text(Some(&format!("{:05}", (index + frame * 7) % 100_000)));
        }
        self.fixture.lay_out();
    }

    fn with_paragraphs() -> Self {
        let mut this = Self::new();
        let panel = WrapPanel::new();
        for index in 0..LABELS / 4 {
            let text = TextBlock::new();
            text.set_font_size(11.0);
            text.set_text_wrapping(TextWrapping::Wrap);
            text.set_width(150.0 + (index % 5) as f64 * 10.0);
            text.set_text(Some("A paragraph that wraps over a few lines and keeps its layout while its width holds."));
            panel.children().add(&text);
        }
        this.fixture.show(Some(&panel.clone().upcast::<Control>()));
        this.fixture.lay_out();
        this.panel = Some(panel);
        this
    }

    fn narrow_and_lay_out(&mut self) {
        let frame = self.frame.get() + 1;
        self.frame.set(frame);
        let panel = self.panel.clone().expect("a panel");
        panel.set_width(if frame % 2 == 0 { 1200.0 } else { 1180.0 });
        self.fixture.lay_out();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("trees", "Text").min_iteration_time(200);
    let parameters = format!("Labels={LABELS}");
    class.benchmark("create_and_measure", parameters.clone(), Text::new, Text::create_and_measure).baseline();
    class.benchmark("change_and_lay_out", parameters.clone(), Text::with_stack, Text::change_and_lay_out);
    class.benchmark("narrow_and_lay_out", parameters, Text::with_paragraphs, Text::narrow_and_lay_out);
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "Text");
    }
}
