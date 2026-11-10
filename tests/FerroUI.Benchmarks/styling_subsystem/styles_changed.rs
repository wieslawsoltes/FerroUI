//! Styles added to and removed from a host above an attached, styled wall, with the styling pass
//! that follows: an added style takes the styles off every element below the host, and each of
//! them evaluates the styles in scope again on its next pass; a removed style is taken off the
//! elements that hold it and nothing is evaluated again. `trees::Propagation.styles_changed`
//! measures the change alone, with a style that matches nothing; here the style sets the `Tag`
//! of the texts and the measurement ends when every element is styled again.
//!
//! - `add_to_root`: a style added to the root: every element of the wall is restyled;
//! - `remove_from_root`: the style removed from the root: the walk that takes its instances off;
//! - `add_to_section`: the style added to one of ten panels that share the tiles: the elements
//!   of that panel are restyled, the pass of the others finds them styled;
//! - `add_range_to_root`, `add_each_to_root`: 32 styles added to the root in one change and one
//!   by one. The elements are restyled once either way; adding them one by one walks the wall
//!   once for each style.

use super::wall::{tag_setter, Wall, CONTROLS};
use crate::harness::Registry;
use ferroui_base::styling::{IStyle, Selectors, Style, Styles};
use ferroui_base::Ref;
use ferroui_controls::{Button, TextBlock};
use std::rc::Rc;

/// The panels that share the tiles.
const SECTIONS: usize = 10;
/// The styles of the benchmarks that add several.
const STYLES: usize = 32;

pub struct StylesChanged {
    wall: Wall,
    /// The styles of the host the benchmark changes.
    host: Ref<Styles>,
    /// The styles added and removed: the first selects the texts, the others select a type the
    /// wall does not hold.
    styles: Vec<Ref<Style>>,
}

impl StylesChanged {
    /// An attached wall and `count` styles that are not in a collection. The benchmark changes
    /// the styles of the root, or with `section` those of the first panel.
    fn new(count: usize, section: bool) -> Self {
        let wall = Wall::with_sections(CONTROLS, SECTIONS);
        let host = if section { wall.sections[0].styles() } else { wall.fixture.root.styles() };
        let mut styles = vec![Style::with_setters(Selectors::of_type::<TextBlock>(), [tag_setter()])];
        while styles.len() < count {
            styles.push(Style::with_setters(Selectors::of_type::<Button>(), [tag_setter()]));
        }
        wall.attach();
        let styled = if section { CONTROLS / SECTIONS } else { CONTROLS };
        let mut this = Self { wall, host, styles };
        this.expect(0, "before the styles are added");
        this.add_each();
        this.expect(styled, "after the styles are added one by one");
        this.remove();
        this.expect(0, "after the styles are removed");
        this.add_range();
        this.expect(styled, "after the styles are added in one change");
        this.remove();
        this.wall.fixture.lay_out();
        this
    }

    fn expect(&self, texts: usize, when: &str) {
        let found = self.wall.tagged_texts();
        if found != texts {
            panic!("Invalid benchmark state: {found} texts are styled {when}, {texts} are expected");
        }
    }

    /// Adds the styles one by one, then styles the wall.
    fn add_each(&mut self) {
        for style in &self.styles {
            self.host.add(style);
        }
        self.wall.style_all();
    }

    /// Adds the styles in one change, then styles the wall.
    fn add_range(&mut self) {
        self.host.add_range(self.styles.iter().map(|style| Rc::<dyn IStyle>::from(style)));
        self.wall.style_all();
    }

    /// Removes the styles that are in the collection.
    fn remove(&mut self) {
        for style in &self.styles {
            self.host.remove(style);
        }
    }

    /// The state before a measured addition: the styles are not in the collection.
    fn prepare_add(&mut self) {
        self.remove();
        self.wall.fixture.lay_out();
    }

    /// The state before a measured removal: the styles are in the collection and applied.
    fn prepare_remove(&mut self) {
        if !self.host.contains(&self.styles[0]) {
            self.add_range();
        }
        self.wall.fixture.lay_out();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling_subsystem", "StylesChanged").min_iteration_time(200);
    let parameters = format!("Controls={CONTROLS}");
    class.benchmark_with_iteration_setup(
        "add_to_root",
        parameters.clone(),
        || StylesChanged::new(1, false),
        StylesChanged::prepare_add,
        StylesChanged::add_each,
    );
    class.benchmark_with_iteration_setup(
        "remove_from_root",
        parameters.clone(),
        || StylesChanged::new(1, false),
        StylesChanged::prepare_remove,
        StylesChanged::remove,
    );
    class.benchmark_with_iteration_setup(
        "add_to_section",
        format!("Controls={CONTROLS}, Sections={SECTIONS}"),
        || StylesChanged::new(1, true),
        StylesChanged::prepare_add,
        StylesChanged::add_each,
    );
    let parameters = format!("Controls={CONTROLS}, Styles={STYLES}");
    class.benchmark_with_iteration_setup(
        "add_range_to_root",
        parameters.clone(),
        || StylesChanged::new(STYLES, false),
        StylesChanged::prepare_add,
        StylesChanged::add_range,
    );
    class.benchmark_with_iteration_setup(
        "add_each_to_root",
        parameters,
        || StylesChanged::new(STYLES, false),
        StylesChanged::prepare_add,
        StylesChanged::add_each,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "StylesChanged");
    }
}
