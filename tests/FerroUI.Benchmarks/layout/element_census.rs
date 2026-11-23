//! What it costs to construct one element, class by class: an element of each class of the
//! chain from a visual to a control, and of the classes most trees are made of. Nothing is set
//! on the element and it is not attached, so the difference between a class and its base is what
//! the class adds to the construction of every element of it (experiment #756).
//!
//! - `construct`: a thousand elements of the class constructed and kept; the time and the
//!   instructions are those of one. The elements are dropped between the iterations;
//! - `drop`: a thousand elements of the class, with nothing set, dropped;
//! - `set`: values set on a thousand new elements, the time and the instructions of one
//!   element: the first value of a border (its width), the five a tile sets on its border, the
//!   three a label sets on its text block, and the same five and three followed by the drop of
//!   the element, which then has values to give back.

use crate::harness::Registry;
use ferroui_base::interactivity::Interactive;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::Brushes;
use ferroui_base::{CornerRadius, FerroObject, Ref, Thickness, Visual};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Border, Button, Canvas, ContentControl, Control, Decorator, Grid, Panel, StackPanel, TextBlock};

const ELEMENTS: usize = 1_000;

pub struct ElementCensus {
    construct: fn() -> Ref<FerroObject>,
    kept: Vec<Ref<FerroObject>>,
    _app: UnitTestApplicationScope,
}

impl ElementCensus {
    fn new(construct: fn() -> Ref<FerroObject>) -> Self {
        let app = UnitTestApplication::start(TestServices::styled_window());
        // The class is initialised before the first iteration.
        drop(construct());
        Self { construct, kept: Vec::with_capacity(ELEMENTS), _app: app }
    }

    fn clear(&mut self) {
        self.kept.clear();
    }

    fn construct(&mut self) {
        for _ in 0..ELEMENTS {
            self.kept.push((self.construct)());
        }
    }

    /// A thousand new elements, kept.
    fn fill(&mut self) {
        self.kept.clear();
        self.construct();
    }

    fn drop_all(&mut self) {
        self.kept.clear();
    }

    fn set(&mut self, set: fn(&Ref<FerroObject>)) {
        for element in &self.kept {
            set(element);
        }
    }
}

/// The first value of an element.
fn border_width(element: &Ref<FerroObject>) {
    element.downcast_ref::<Border>().expect("a border").set_width(5.0);
}

/// What a tile sets on its border.
fn border_five(element: &Ref<FerroObject>) {
    let border = element.downcast_ref::<Border>().expect("a border");
    border.set_width(5.0);
    border.set_height(4.0);
    border.set_corner_radius(CornerRadius::uniform(2.0));
    border.set_padding(Thickness::uniform(0.24));
    border.set_background(Some(Brushes::blue()));
}

/// What a label sets on its text block.
fn text_three(element: &Ref<FerroObject>) {
    let text = element.downcast_ref::<TextBlock>().expect("a text block");
    text.set_font_size(1.2);
    text.set_foreground(Some(Brushes::white()));
    text.set_text(Some("00042"));
}

pub fn register(registry: &mut Registry) {
    let classes: [(&str, fn() -> Ref<FerroObject>); 15] = [
        ("Visual", || Visual::new().upcast()),
        ("Layoutable", || Layoutable::new().upcast()),
        ("Interactive", || Interactive::new().upcast()),
        ("Control", || Control::new().upcast()),
        ("Decorator", || Decorator::new().upcast()),
        ("Border", || Border::new().upcast()),
        ("Panel", || Panel::new().upcast()),
        ("StackPanel", || StackPanel::new().upcast()),
        ("Canvas", || Canvas::new().upcast()),
        ("Grid", || Grid::new().upcast()),
        ("TextBlock", || TextBlock::new().upcast()),
        ("Rectangle", || Rectangle::new().upcast()),
        ("ContentPresenter", || ContentPresenter::new().upcast()),
        ("ContentControl", || ContentControl::new().upcast()),
        ("Button", || Button::new().upcast()),
    ];
    let mut class = registry.class("layout", "ElementCensus").min_iteration_time(200);
    for (name, construct) in classes {
        class
            .benchmark_with_iteration_setup(
                "construct",
                format!("Class={name}"),
                move || ElementCensus::new(construct),
                ElementCensus::clear,
                ElementCensus::construct,
            )
            .operations_per_invoke(ELEMENTS as u32);
    }
    for (name, construct) in [classes[5], classes[7], classes[10], classes[14]] {
        class
            .benchmark_with_iteration_setup(
                "drop",
                format!("Class={name}"),
                move || ElementCensus::new(construct),
                ElementCensus::fill,
                ElementCensus::drop_all,
            )
            .operations_per_invoke(ELEMENTS as u32);
    }
    let sets: [(&str, usize, fn(&Ref<FerroObject>)); 3] = [
        ("Class=Border, Values=1", 5, border_width),
        ("Class=Border, Values=5", 5, border_five),
        ("Class=TextBlock, Values=3", 10, text_three),
    ];
    for (parameters, class_index, set) in sets {
        let construct = classes[class_index].1;
        class
            .benchmark_with_iteration_setup(
                "set",
                parameters,
                move || ElementCensus::new(construct),
                ElementCensus::fill,
                move |census: &mut ElementCensus| census.set(set),
            )
            .operations_per_invoke(ELEMENTS as u32);
        if parameters.ends_with("Values=1") {
            continue;
        }
        class
            .benchmark_with_iteration_setup(
                "set_and_drop",
                parameters,
                move || ElementCensus::new(construct),
                ElementCensus::fill,
                move |census: &mut ElementCensus| {
                    census.set(set);
                    census.drop_all();
                },
            )
            .operations_per_invoke(ELEMENTS as u32);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "ElementCensus");
    }
}
