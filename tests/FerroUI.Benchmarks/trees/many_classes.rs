//! An inherited value passed down a panel of elements of many classes. The change asks, for every
//! element, whether its class hears the property, and the answers of the classes asked last are
//! kept in eight places:
//!
//! - `font_family[Classes=8]`: as many classes as there are places;
//! - `font_family[Classes=12]`: more classes than places, so that a walk keeps asking for classes
//!   whose answers were pushed out.
//!
//! A layout pass runs before each change, outside the measurement.

use super::fixture::Fixture;
use crate::harness::Registry;
use ferroui_base::media::FontFamily;
use ferroui_base::Ref;
use ferroui_controls::documents::TextElement;
use ferroui_controls::{
    Border, Canvas, Control, Decorator, DockPanel, Grid, Image, Panel, RelativePanel, StackPanel, TextBlock, Viewbox,
    WrapPanel,
};
use std::cell::Cell;

/// How many times the panel holds one element of each class.
const ROUNDS: usize = 100;

/// An element of the class at `index`; the panel that holds them is one more class.
fn element(index: usize) -> Ref<Control> {
    match index {
        0 => Border::new().upcast(),
        1 => Decorator::new().upcast(),
        2 => Canvas::new().upcast(),
        3 => Grid::new().upcast(),
        4 => StackPanel::new().upcast(),
        5 => DockPanel::new().upcast(),
        6 => TextBlock::new().upcast(),
        7 => WrapPanel::new().upcast(),
        8 => RelativePanel::new().upcast(),
        9 => Image::new().upcast(),
        10 => Viewbox::new().upcast(),
        _ => unreachable!("the benchmark has eleven classes below the panel"),
    }
}

pub struct ManyClasses {
    fixture: Fixture,
    panel: Ref<Control>,
    flip: Cell<bool>,
}

impl ManyClasses {
    /// A panel of `classes` classes, itself included.
    fn new(classes: usize) -> Self {
        let fixture = Fixture::new();
        let panel = Panel::new();
        for _ in 0..ROUNDS {
            for index in 0..classes - 1 {
                panel.children().add(&element(index));
            }
        }
        let panel: Ref<Control> = panel.upcast();
        fixture.show(Some(&panel));
        fixture.lay_out();
        Self { fixture, panel, flip: Cell::new(false) }
    }

    fn settle(&mut self) {
        self.fixture.lay_out();
    }

    fn change(&mut self) {
        let value = !self.flip.get();
        self.flip.set(value);
        let family = if value { "Roboto" } else { "Inter" };
        TextElement::set_font_family_on(&self.panel, FontFamily::new(family));
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("trees", "ManyClasses").min_iteration_time(200);
    for classes in [8, 12] {
        class.benchmark_with_iteration_setup(
            "font_family",
            format!("Classes={classes}"),
            move || ManyClasses::new(classes),
            ManyClasses::settle,
            ManyClasses::change,
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "ManyClasses");
    }
}
