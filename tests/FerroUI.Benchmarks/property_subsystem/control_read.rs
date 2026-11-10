//! One read of a property of a control, by where the value comes from:
//!
//! - `hot_copy`: `Width`, which a layoutable keeps a copy of beside its fields (section 9.9);
//! - `unset_empty_store`: `Opacity` of a control that holds no value at all: the default of the
//!   property, found without a value to look through;
//! - `unset_beside_values`: `Opacity` of a control that holds six other values: the values are
//!   looked through first;
//! - `local_value`: `Opacity` set on the control, beside the six other values;
//! - `non_copy_value`: `Background`, a shared brush: the read hands out one more owner of it;
//! - `inherited_unheld`: `UseLayoutRounding`, which no control of the tree holds: the default of an
//!   inherited property;
//! - `inherited_held`: the font size, which the panel above the controls holds: the value of the
//!   nearest ancestor that has one, kept by the store of the control between two changes of the
//!   tree (section 9.3);
//! - `attached_unset`: `Canvas.Left` of a control that never had one.
//!
//! The controls are the children of one panel, shown and laid out. Each invocation reads the
//! property of every control, as often as it takes to make ten thousand reads.

use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::{CornerRadius, Ref, Thickness};
use ferroui_controls::documents::TextElement;
use ferroui_controls::{Border, Canvas, StackPanel};

/// The reads of one invocation.
const READS: usize = 10_000;

pub struct ControlRead {
    _fixture: Fixture,
    controls: Vec<Ref<Border>>,
    /// How often the controls are walked in one invocation.
    rounds: usize,
}

impl ControlRead {
    /// `objects` controls in a panel that holds a font size, shown and laid out. A control with
    /// `values` holds six local values; `opacity` gives it a seventh.
    fn new(objects: usize, values: bool, opacity: bool) -> Self {
        let fixture = Fixture::new();
        let panel = StackPanel::new();
        TextElement::set_font_size_on(&panel, 20.0);
        let controls: Vec<Ref<Border>> = (0..objects)
            .map(|_| {
                let control = Border::new();
                if values {
                    control.set_width(10.0);
                    control.set_height(4.0);
                    control.set_margin(Thickness::uniform(1.0));
                    control.set_background(Some(fixture.brushes.tile.clone()));
                    control.set_border_thickness(Thickness::uniform(1.0));
                    control.set_corner_radius(CornerRadius::uniform(2.0));
                }
                if opacity {
                    control.set_opacity(0.5);
                }
                panel.children().add(&control);
                control
            })
            .collect();
        fixture.show(Some(&panel.upcast()));
        fixture.lay_out();
        Self { _fixture: fixture, controls, rounds: READS / objects }
    }

    fn bare(objects: usize) -> Self {
        Self::new(objects, false, false)
    }

    fn with_values(objects: usize) -> Self {
        Self::new(objects, true, false)
    }

    fn with_opacity(objects: usize) -> Self {
        Self::new(objects, true, true)
    }

    /// Reads a number of every control, `rounds` times.
    fn sum(&self, read: impl Fn(&Ref<Border>) -> f64) -> f64 {
        let mut sum = 0.0;
        for _ in 0..self.rounds {
            for control in &self.controls {
                sum += read(control);
            }
        }
        sum
    }

    fn width(&mut self) -> f64 {
        self.sum(|control| control.width())
    }

    fn opacity(&mut self) -> f64 {
        self.sum(|control| control.opacity())
    }

    fn background(&mut self) -> f64 {
        self.sum(|control| if control.background().is_some() { 1.0 } else { 0.0 })
    }

    fn use_layout_rounding(&mut self) -> f64 {
        self.sum(|control| if control.use_layout_rounding() { 1.0 } else { 0.0 })
    }

    fn font_size(&mut self) -> f64 {
        self.sum(|control| TextElement::get_font_size(control))
    }

    fn canvas_left(&mut self) -> f64 {
        self.sum(|control| Canvas::get_left(control))
    }
}

type Setup = fn(usize) -> ControlRead;
type Body = fn(&mut ControlRead) -> f64;

const READERS: [(&str, Setup, Body); 8] = [
    ("hot_copy", ControlRead::with_values, ControlRead::width),
    ("unset_empty_store", ControlRead::bare, ControlRead::opacity),
    ("unset_beside_values", ControlRead::with_values, ControlRead::opacity),
    ("local_value", ControlRead::with_opacity, ControlRead::opacity),
    ("non_copy_value", ControlRead::with_values, ControlRead::background),
    ("inherited_unheld", ControlRead::with_values, ControlRead::use_layout_rounding),
    ("inherited_held", ControlRead::with_values, ControlRead::font_size),
    ("attached_unset", ControlRead::with_values, ControlRead::canvas_left),
];

/// A walk that stays in the first cache of the processor, and one that does not.
const OBJECTS: [usize; 2] = [16, 10_000];

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("property_subsystem", "ControlRead").min_iteration_time(200);
    for (name, setup, body) in READERS {
        for objects in OBJECTS {
            class
                .benchmark(name, format!("Objects={objects}"), move || setup(objects), body)
                .operations_per_invoke(READS as u32);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "ControlRead");
    }
}
