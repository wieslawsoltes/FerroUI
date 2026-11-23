//! One matching style of the root, one kind of selector: a styling pass of the texts of an
//! attached wall. Every text matches, so each benchmark costs the walk of the style hosts, the
//! evaluation of that selector against a text, and one style instance attached to it.
//!
//! - `none`: no style of the root at all: the floor of a styling pass, the walk of the style
//!   hosts up to the global styles of the application (sections 9.21 and 9.25);
//! - `type`: the type of the control; a style without an activator and with plain values is one
//!   instance shared by every text;
//! - `type_class`, `type_two_classes`: classes the texts carry; a class selector gives every text
//!   an activator that follows its classes;
//! - `pseudo_class`: a pseudo-class that is set on the texts;
//! - `child`: the logical parent of the text, by type;
//! - `descendant_two_levels`, `descendant_four_levels`: an ancestor two and four levels up, by
//!   type: the walk up the logical tree for every text;
//! - `template`: the templated parent of the text, by type; the texts name their panel as their
//!   templated parent, which no control template has to build;
//! - `nth_child`: the position of the text among the children of its panel;
//! - `not`: a class the texts do not carry, negated;
//! - `or`: two class selectors, of which the first matches;
//! - `property_equals`: a property of the text that has the value of the selector.

use super::wall::{tag_setter, Wall, CONTROLS};
use crate::harness::Registry;
use ferroui_base::styling::{Selector, Selectors, Style};
use ferroui_base::Visual;
use ferroui_controls::{Canvas, Panel, StackPanel, TextBlock};

/// Gives the wall what a kind of selector matches on and returns the selector of the root's style.
type Arrange = fn(&Wall) -> Option<Selector>;

fn text() -> Selector {
    Selectors::of_type::<TextBlock>()
}

fn add_classes(wall: &Wall, names: &[&str]) {
    for tile in &wall.tiles {
        for name in names {
            tile.text.classes().add(name);
        }
    }
}

const KINDS: [(&str, Arrange); 13] = [
    ("none", |_| None),
    ("type", |_| Some(text())),
    ("type_class", |wall| {
        add_classes(wall, &["value"]);
        Some(text().class("value"))
    }),
    ("type_two_classes", |wall| {
        add_classes(wall, &["value", "strong"]);
        Some(text().class("value").class("strong"))
    }),
    ("pseudo_class", |wall| {
        for tile in &wall.tiles {
            tile.text.set_pseudo_class(":pointerover", true);
        }
        Some(text().class(":pointerover"))
    }),
    ("child", |_| Some(Selectors::of_type::<Panel>().child().of_type::<TextBlock>())),
    ("descendant_two_levels", |_| Some(Selectors::of_type::<StackPanel>().descendant().of_type::<TextBlock>())),
    ("descendant_four_levels", |_| Some(Selectors::of_type::<Canvas>().descendant().of_type::<TextBlock>())),
    ("template", |wall| {
        for tile in &wall.tiles {
            tile.text.set_templated_parent(&tile.line);
        }
        Some(Selectors::of_type::<Panel>().template().of_type::<TextBlock>())
    }),
    ("nth_child", |_| Some(text().nth_child(2, 1))),
    ("not", |_| Some(text().not(Selectors::class(None, "muted")))),
    ("or", |wall| {
        add_classes(wall, &["value"]);
        Some(Selectors::or([text().class("value"), text().class("label")]))
    }),
    ("property_equals", |_| Some(text().property_equals(Visual::is_visible_property(), true))),
];

/// An attached wall whose root holds the style of one kind of selector.
fn wall(arrange: Arrange) -> Wall {
    let mut wall = Wall::new(CONTROLS);
    let selector = arrange(&wall);
    let styled = selector.is_some();
    if let Some(selector) = selector {
        wall.fixture.root.styles().add(&Style::with_setters(selector, [tag_setter()]));
    }
    wall.attach();
    wall.check_restyle(styled);
    wall
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling_subsystem", "SelectorMatch").min_iteration_time(200);
    let parameters = format!("Controls={CONTROLS}");
    for (name, arrange) in KINDS {
        let descriptor = class.benchmark_with_iteration_setup(
            name,
            parameters.clone(),
            move || wall(arrange),
            Wall::prepare_restyle,
            |wall| wall.style_texts(),
        );
        if name == "type" {
            descriptor.baseline();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "SelectorMatch");
    }
}
