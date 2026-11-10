//! Styles on a wall of tiles: styles of the root that select the tiles' elements by type and class.
//!
//! - `attach`: a built wall whose elements carry classes shown in a root with six such styles;
//!   compare with `trees::Lifetime.attach`, whose root has none of its own;
//! - `toggle_class`: a class added to and removed from every tile of an attached wall, which
//!   activates and deactivates a style of the tiles;
//! - `resource_changed`: a resource of the root replaced; every element below is told.

use super::fixture::{tile_parts, Brushes, Fixture, TILES};
use crate::harness::Registry;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{Border, Canvas, Control, StackPanel, TextBlock};
use std::cell::Cell;

pub struct Styling {
    fixture: Fixture,
    tiles: usize,
    pending: Option<Ref<Canvas>>,
    built: Vec<Ref<Border>>,
    flip: Cell<bool>,
}

fn styles(fixture: &Fixture) {
    let b = &fixture.brushes;
    let styles = fixture.root.styles();
    styles.add(&Style::with_setters(
        Selectors::of_type::<Border>().class("tile"),
        [Setter::new(Border::border_thickness_property(), Thickness::uniform(0.0))],
    ));
    styles.add(&Style::with_setters(
        Selectors::of_type::<Border>().class("tile").class("hot"),
        [Setter::new(Border::background_property(), Some(b.other.clone()))],
    ));
    styles.add(&Style::with_setters(
        Selectors::of_type::<Border>().class("dot"),
        [Setter::new(Border::border_thickness_property(), Thickness::uniform(0.0))],
    ));
    styles.add(&Style::with_setters(
        Selectors::of_type::<TextBlock>().class("label"),
        [Setter::new(TextBlock::letter_spacing_property(), 0.0)],
    ));
    styles.add(&Style::with_setters(
        Selectors::of_type::<TextBlock>().class("value"),
        [Setter::new(TextBlock::letter_spacing_property(), 0.0)],
    ));
    styles.add(&Style::with_setters(
        Selectors::of_type::<StackPanel>().class("line"),
        [Setter::new(StackPanel::spacing_property(), 1.0)],
    ));
}

/// A tile whose elements carry the classes the styles select.
fn classed_tile(index: usize, brushes: &Brushes) -> Ref<Border> {
    let parts = tile_parts(index, brushes);
    parts.tile.classes().add("tile");
    parts.name.classes().add("label");
    parts.line.classes().add("line");
    parts.spot.classes().add("dot");
    parts.value.classes().add("value");
    parts.line.children().add(&parts.spot);
    parts.line.children().add(&parts.value);
    parts.content.children().add(&parts.name);
    parts.content.children().add(&parts.line);
    parts.tile.set_child(&parts.content);
    parts.tile
}

impl Styling {
    fn new(tiles: usize) -> Self {
        let fixture = Fixture::new();
        styles(&fixture);
        fixture.root.resources().add("Accent", None);
        Self { fixture, tiles, pending: None, built: Vec::new(), flip: Cell::new(false) }
    }

    fn prepare_detached(&mut self) {
        self.fixture.show(None);
        // The layout queue holds what was detached until its next pass: freed here, unmeasured.
        self.fixture.lay_out();
        let canvas = Canvas::new();
        self.built.clear();
        for index in 0..self.tiles {
            let tile = classed_tile(index, &self.fixture.brushes);
            canvas.children().add(&tile);
            self.built.push(tile);
        }
        self.pending = Some(canvas);
    }

    fn attach(&mut self) {
        let canvas = self.pending.clone().expect("a wall").upcast::<Control>();
        self.fixture.show(Some(&canvas));
    }

    fn new_attached(tiles: usize) -> Self {
        let mut this = Self::new(tiles);
        this.prepare_detached();
        this.attach();
        this.fixture.lay_out();
        this
    }

    fn settle(&mut self) {
        self.fixture.lay_out();
    }

    fn toggle_class(&mut self) {
        let hot = !self.flip.get();
        self.flip.set(hot);
        for tile in &self.built {
            tile.classes().set("hot", hot);
        }
    }

    fn resource_changed(&mut self) {
        let value = !self.flip.get();
        self.flip.set(value);
        let accent: ferroui_base::BoxedValue = std::rc::Rc::new(i32::from(value));
        self.fixture.root.resources().set("Accent", Some(accent));
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("trees", "Styling").min_iteration_time(200);
    let parameters = format!("Tiles={TILES}");
    class.benchmark_with_iteration_setup("attach", parameters.clone(), || Styling::new(TILES), Styling::prepare_detached, Styling::attach);
    class.benchmark_with_iteration_setup("toggle_class", parameters.clone(), || Styling::new_attached(TILES), Styling::settle, Styling::toggle_class);
    class.benchmark_with_iteration_setup(
        "resource_changed",
        parameters,
        || Styling::new_attached(TILES),
        Styling::settle,
        Styling::resource_changed,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "Styling");
    }
}
