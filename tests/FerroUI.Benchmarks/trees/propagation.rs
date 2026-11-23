//! Values passed down an attached, laid out wall of tiles from a panel above it: each benchmark
//! changes one value of the panel (alternately to one of two values) and the change reaches every
//! element below. A layout pass runs before each change, outside the measurement.
//!
//! - `font_family`, `font_weight`: inherited properties none of the tiles sets (section 9.3);
//! - `is_enabled`: the effective enabled state (section 9.21);
//! - `is_visible`: hiding and showing the wall;
//! - `flow_direction`: the mirror transforms;
//! - `data_context`: the data context, an inherited property every element has;
//! - `theme_variant`: the theme variant of a scope above the wall: its resources and the controls
//!   that follow them;
//! - `styles_changed`: a style added to and removed from the root's styles.

use super::fixture::{wall, Fixture, TILES};
use crate::harness::Registry;
use ferroui_base::media::{FlowDirection, FontFamily, FontWeight};
use ferroui_base::styling::{Selectors, Setter, Style, ThemeVariant};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::documents::TextElement;
use ferroui_controls::{Border, Canvas, Control, ThemeVariantScope};
use std::cell::Cell;
use std::rc::Rc;

pub struct Propagation {
    fixture: Fixture,
    scope: Ref<ThemeVariantScope>,
    canvas: Ref<Canvas>,
    style: Ref<Style>,
    flip: Cell<bool>,
}

impl Propagation {
    fn new(tiles: usize) -> Self {
        let fixture = Fixture::new();
        let canvas = wall(tiles, &fixture.brushes);
        let scope = ThemeVariantScope::new();
        scope.set_requested_theme_variant(Some(ThemeVariant::light()));
        scope.set_child(&canvas);
        fixture.show(Some(&scope.clone().upcast::<Control>()));
        fixture.lay_out();
        let style = Style::with_setters(
            Selectors::of_type::<Border>().class("never"),
            [Setter::new(Border::background_property(), Some(fixture.brushes.other.clone()))],
        );
        Self { fixture, scope, canvas, style, flip: Cell::new(false) }
    }

    /// The next of the two values.
    fn flip(&self) -> bool {
        let value = !self.flip.get();
        self.flip.set(value);
        value
    }

    fn settle(&mut self) {
        self.fixture.lay_out();
    }

    fn control(&self) -> Ref<Control> {
        self.canvas.clone().upcast()
    }
}

type Body = fn(&mut Propagation);

const CHANGES: [(&str, Body); 8] = [
    ("font_family", |b| {
        let family = if b.flip() { "Roboto" } else { "Inter" };
        TextElement::set_font_family_on(&b.control(), FontFamily::new(family));
    }),
    ("font_weight", |b| {
        let weight = if b.flip() { FontWeight::Bold } else { FontWeight::Normal };
        TextElement::set_font_weight_on(&b.control(), weight);
    }),
    ("is_enabled", |b| {
        let enabled = !b.flip();
        b.canvas.set_is_enabled(enabled);
    }),
    ("is_visible", |b| {
        let visible = !b.flip();
        b.canvas.set_is_visible(visible);
    }),
    ("flow_direction", |b| {
        let direction = if b.flip() { FlowDirection::RightToLeft } else { FlowDirection::LeftToRight };
        b.canvas.set_flow_direction(direction);
    }),
    ("data_context", |b| {
        let value: BoxedValue = Rc::new(i32::from(b.flip()));
        b.canvas.set_data_context(Some(value));
    }),
    ("theme_variant", |b| {
        let variant = if b.flip() { ThemeVariant::dark() } else { ThemeVariant::light() };
        b.scope.set_requested_theme_variant(Some(variant));
    }),
    ("styles_changed", |b| {
        let styles = b.fixture.root.styles();
        if b.flip() {
            styles.add(&b.style);
        } else {
            styles.remove(&b.style);
        }
    }),
];

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("trees", "Propagation").min_iteration_time(200);
    let parameters = format!("Tiles={TILES}");
    for (name, body) in CHANGES {
        class.benchmark_with_iteration_setup(name, parameters.clone(), || Propagation::new(TILES), Propagation::settle, body);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "Propagation");
    }
}
