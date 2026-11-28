//! An object that gets or loses its inheritance parent below ancestors that hold inherited values
//! (the font family, size and weight, the foreground and the theme variant, as the theme of a
//! window gives them to it): each inherited value that differs is a change for the object.
//!
//! - `definitions_replaced`: the column definitions of a grid replaced by two new star columns,
//!   as a bar written as a grid of two columns does when its value changes: two definitions leave
//!   the grid and two enter it;
//! - `leaf_moved`: a border moved from one parent to another, which hears none of the values;
//! - `text_moved`: a text block moved the same way, which hears the font and the foreground.
//!
//! The grids and the parents are attached and styled; a layout pass runs before each iteration,
//! outside the measurement. Each invocation does the operation once for every object.

use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::media::{FontFamily, FontWeight};
use ferroui_base::Ref;
use ferroui_controls::documents::TextElement;
use ferroui_controls::{Border, ColumnDefinitions, Control, Grid, StackPanel, TextBlock};

const OBJECTS: usize = 1_000;

pub struct InheritanceParent {
    fixture: Fixture,
    grids: Vec<Ref<Grid>>,
    /// The two parents of each object that moves.
    parents: Vec<[Ref<Border>; 2]>,
    moved: Vec<Ref<Control>>,
    /// Which of the two states the next invocation makes.
    flip: bool,
}

impl InheritanceParent {
    fn with(grids: usize, moved: impl Fn() -> Ref<Control>, count: usize) -> Self {
        let fixture = Fixture::new();
        let panel = StackPanel::new();
        // The values the theme of a window gives it, which every element below inherits.
        let control: Ref<Control> = panel.clone().upcast();
        TextElement::set_font_family_on(&control, FontFamily::new("Inter"));
        TextElement::set_font_size_on(&control, 14.0);
        TextElement::set_font_weight_on(&control, FontWeight::Medium);
        TextElement::set_foreground_on(&control, Some(fixture.brushes.label.clone()));
        let grids: Vec<_> = (0..grids).map(|_| Grid::new()).collect();
        for grid in &grids {
            panel.children().add(grid);
        }
        let parents: Vec<_> = (0..count).map(|_| [Border::new(), Border::new()]).collect();
        let moved: Vec<_> = (0..count).map(|_| moved()).collect();
        for (parents, moved) in parents.iter().zip(&moved) {
            for parent in parents {
                panel.children().add(parent);
            }
            parents[0].set_child(moved);
        }
        fixture.show(Some(&panel.upcast::<Control>()));
        fixture.lay_out();
        Self { fixture, grids, parents, moved, flip: false }
    }

    fn grids() -> Self {
        Self::with(OBJECTS, || Border::new().upcast(), 0)
    }

    fn borders() -> Self {
        Self::with(0, || Border::new().upcast(), OBJECTS)
    }

    fn texts() -> Self {
        Self::with(
            0,
            || {
                let text = TextBlock::new();
                text.set_text(Some("Text"));
                text.upcast()
            },
            OBJECTS,
        )
    }

    fn settle(&mut self) {
        self.fixture.lay_out();
    }

    fn replace_definitions(&mut self) {
        self.flip = !self.flip;
        let columns = if self.flip { "0.25*,0.75*" } else { "0.5*,0.5*" };
        for grid in &self.grids {
            grid.set_column_definitions(ColumnDefinitions::parse(columns).expect("two star columns"));
        }
    }

    fn move_objects(&mut self) {
        self.flip = !self.flip;
        let (from, to) = if self.flip { (0, 1) } else { (1, 0) };
        for (parents, moved) in self.parents.iter().zip(&self.moved) {
            parents[from].set_child(None::<Ref<Control>>);
            parents[to].set_child(moved);
        }
    }
}

type Setup = fn() -> InheritanceParent;
type Body = fn(&mut InheritanceParent);

const CHANGES: [(&str, Setup, Body); 3] = [
    ("definitions_replaced", InheritanceParent::grids, InheritanceParent::replace_definitions),
    ("leaf_moved", InheritanceParent::borders, InheritanceParent::move_objects),
    ("text_moved", InheritanceParent::texts, InheritanceParent::move_objects),
];

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("property_subsystem", "InheritanceParent").min_iteration_time(200);
    for (name, setup, body) in CHANGES {
        class
            .benchmark_with_iteration_setup(name, format!("Objects={OBJECTS}"), setup, InheritanceParent::settle, body)
            .operations_per_invoke(OBJECTS as u32);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "InheritanceParent");
    }
}
