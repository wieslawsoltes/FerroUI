//! One write of a property of a control, by the kind of the property and of the write:
//!
//! - `local_changed`: `Opacity` set to another value: the value is stored and the change is told
//!   to the class handlers of the whole hierarchy of the control;
//! - `local_unchanged`: `Opacity` set to the value it has: nothing is told;
//! - `local_changed_affects_measure`: `Width`: the change also invalidates the measure;
//! - `non_copy_changed`: `Background`, set to another shared brush;
//! - `attached_changed`: `Canvas.Left`;
//! - `inherited_changed`: the font size, on a control without children: the change of an inherited
//!   value starts a new generation of the inherited lookups (section 9.3);
//! - `style_priority_set_and_dispose`: `Opacity` set with the priority of a style and taken back:
//!   such a value is an entry of a value frame of its own, beside the values of the styles;
//! - `set_and_clear`: `Opacity` set and cleared again: the value is created and destroyed.
//!
//! `local_changed` and `local_changed_affects_measure` are measured on a border and on a button,
//! whose hierarchy is three classes deeper. The controls are in no tree, so a write costs what the
//! property system and the class handlers cost and no pass is asked for. Each invocation writes
//! the property of every control once, with a value that alternates between two invocations.

use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Ref, Visual};
use ferroui_controls::documents::TextElement;
use ferroui_controls::{Border, Button, Canvas, Control};

const OBJECTS: usize = 1_000;

pub struct ControlWrite {
    fixture: Fixture,
    controls: Vec<Ref<Control>>,
    /// Which of the two values the next invocation writes.
    flip: bool,
}

impl ControlWrite {
    fn of(create: impl Fn() -> Ref<Control>) -> Self {
        let fixture = Fixture::new();
        let controls = (0..OBJECTS).map(|_| create()).collect();
        let mut this = Self { fixture, controls, flip: false };
        // Every property the benchmarks write has a value before the first measurement, so that a
        // write changes a value and does not create one.
        this.opacity();
        this.width();
        this.canvas_left();
        this.font_size();
        this
    }

    fn borders() -> Self {
        Self::of(|| Border::new().upcast())
    }

    fn buttons() -> Self {
        Self::of(|| Button::new().upcast())
    }

    /// The first of the two values, or the second, in turn.
    fn next(&mut self, first: f64, second: f64) -> f64 {
        self.flip = !self.flip;
        if self.flip {
            first
        } else {
            second
        }
    }

    fn opacity(&mut self) {
        let value = self.next(0.25, 0.75);
        for control in &self.controls {
            control.set_opacity(value);
        }
    }

    fn opacity_unchanged(&mut self) {
        for control in &self.controls {
            control.set_opacity(0.5);
        }
    }

    fn width(&mut self) {
        let value = self.next(10.0, 20.0);
        for control in &self.controls {
            control.set_width(value);
        }
    }

    fn background(&mut self) {
        self.flip = !self.flip;
        let brush = if self.flip { &self.fixture.brushes.tile } else { &self.fixture.brushes.other };
        for control in &self.controls {
            control.set_value(Border::background_property(), Some(brush.clone()));
        }
    }

    fn canvas_left(&mut self) {
        let value = self.next(1.0, 2.0);
        for control in &self.controls {
            Canvas::set_left(control, value);
        }
    }

    fn font_size(&mut self) {
        let value = self.next(12.0, 14.0);
        for control in &self.controls {
            TextElement::set_font_size_on(control, value);
        }
    }

    fn opacity_with_style_priority(&mut self) {
        for control in &self.controls {
            let entry = control.set_value_with_priority(Visual::opacity_property(), 0.5, BindingPriority::Style);
            if let Some(entry) = entry {
                entry.dispose();
            }
        }
    }

    fn opacity_set_and_clear(&mut self) {
        for control in &self.controls {
            control.set_opacity(0.5);
            control.clear_value(Visual::opacity_property());
        }
    }

    /// Controls whose opacity has no local value: what `opacity_with_style_priority` and
    /// `opacity_set_and_clear` start from.
    fn without_opacity(mut self) -> Self {
        for control in &self.controls {
            control.clear_value(Visual::opacity_property());
        }
        self.flip = false;
        self
    }
}

type Setup = fn() -> ControlWrite;
type Body = fn(&mut ControlWrite);

const WRITERS: [(&str, &str, Setup, Body); 10] = [
    ("local_changed", "Border", ControlWrite::borders, ControlWrite::opacity),
    ("local_changed", "Button", ControlWrite::buttons, ControlWrite::opacity),
    ("local_unchanged", "Border", ControlWrite::borders, ControlWrite::opacity_unchanged),
    ("local_changed_affects_measure", "Border", ControlWrite::borders, ControlWrite::width),
    ("local_changed_affects_measure", "Button", ControlWrite::buttons, ControlWrite::width),
    ("non_copy_changed", "Border", ControlWrite::borders, ControlWrite::background),
    ("attached_changed", "Border", ControlWrite::borders, ControlWrite::canvas_left),
    ("inherited_changed", "Border", ControlWrite::borders, ControlWrite::font_size),
    (
        "style_priority_set_and_dispose",
        "Border",
        || ControlWrite::borders().without_opacity(),
        ControlWrite::opacity_with_style_priority,
    ),
    ("set_and_clear", "Border", || ControlWrite::borders().without_opacity(), ControlWrite::opacity_set_and_clear),
];

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("property_subsystem", "ControlWrite").min_iteration_time(200);
    for (name, control, setup, body) in WRITERS {
        class
            .benchmark(name, format!("Control={control}, Objects={OBJECTS}"), setup, body)
            .operations_per_invoke(OBJECTS as u32);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "ControlWrite");
    }
}
