//! Styles switched on and off on an attached wall: the selector of the style matches the type of
//! the texts and a class decides whether it applies, so every text holds an instance of the style
//! with an activator, and a change of the class costs the activators told, the instances switched
//! and the value of each text evaluated again. The styles set the `Tag` of the texts: nothing is
//! laid out or drawn after a switch.
//!
//! - `ancestor_class`: a class added to and removed from the canvas, in turns, with a style that
//!   selects the texts below a canvas of that class: one change of a class, an activator of
//!   every text told (compare `trees::Styling.toggle_class`, which changes a class of every tile);
//! - `pseudo_class_set`: a pseudo-class set on every text, which switches on the style that
//!   selects it; the value of the text goes from unset to the setter's;
//! - `pseudo_class_reset`: the pseudo-class taken off every text; the style is switched off and
//!   the value goes back to unset;
//! - `pseudo_class_set_unobserved`: the pseudo-class set while no style selects it: the change of
//!   the classes alone, the floor of the two above (section 9.21, classes);
//! - `pseudo_class_set_over_style`: the pseudo-class set while a style that is always active sets
//!   the same property below the switching one: the value goes from one setter's to the other's.

use super::wall::{tag, Wall, CONTROLS};
use crate::harness::Registry;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_controls::{Canvas, Control, TextBlock};
use std::cell::Cell;

const POINTER_OVER: &str = ":pointerover";

/// The `Tag` the style that is always active sets.
const PLAIN: i32 = 1;
/// The `Tag` the style that is switched sets.
const SWITCHED: i32 = 2;

pub struct Activation {
    wall: Wall,
    flip: Cell<bool>,
}

impl Activation {
    /// An attached wall whose root holds a style of the texts that a class of the canvas switches.
    fn ancestor_class() -> Self {
        let wall = Wall::new(CONTROLS);
        wall.fixture.root.styles().add(&Style::with_setters(
            Selectors::of_type::<Canvas>().class("dim").descendant().of_type::<TextBlock>(),
            [Setter::new(Control::tag_property(), tag(SWITCHED))],
        ));
        wall.attach();
        let this = Self { wall, flip: Cell::new(false) };
        this.expect(SWITCHED, 0, "before the class is set");
        this.toggle_ancestor_class();
        this.expect(SWITCHED, CONTROLS, "with the class on the canvas");
        this.toggle_ancestor_class();
        this.expect(SWITCHED, 0, "after the class is removed");
        this
    }

    /// An attached wall whose root holds, as asked, a style of the texts that is always active
    /// and, after it, a style of the texts that their pseudo-class switches.
    fn pseudo_class(always_active: bool, switching: bool) -> Self {
        let wall = Wall::new(CONTROLS);
        let styles = wall.fixture.root.styles();
        if always_active {
            styles.add(&Style::with_setters(
                Selectors::of_type::<TextBlock>(),
                [Setter::new(Control::tag_property(), tag(PLAIN))],
            ));
        }
        if switching {
            styles.add(&Style::with_setters(
                Selectors::of_type::<TextBlock>().class(POINTER_OVER),
                [Setter::new(Control::tag_property(), tag(SWITCHED))],
            ));
        }
        wall.attach();
        let this = Self { wall, flip: Cell::new(false) };
        let plain = if always_active { CONTROLS } else { 0 };
        let switched = if switching { CONTROLS } else { 0 };
        this.expect(PLAIN, plain, "before the pseudo-class is set");
        this.expect(SWITCHED, 0, "before the pseudo-class is set");
        this.set_pointer_over(true);
        // The later style wins while it is on.
        this.expect(SWITCHED, switched, "with the pseudo-class set");
        this.expect(PLAIN, if switching { 0 } else { plain }, "with the pseudo-class set");
        this.set_pointer_over(false);
        this.expect(PLAIN, plain, "after the pseudo-class is reset");
        this.expect(SWITCHED, 0, "after the pseudo-class is reset");
        this
    }

    fn expect(&self, value: i32, texts: usize, when: &str) {
        let found = self.wall.texts_with_tag(value);
        if found != texts {
            panic!("Invalid benchmark state: {found} texts have the tag {value} {when}, {texts} are expected");
        }
    }

    fn toggle_ancestor_class(&self) {
        let dim = !self.flip.get();
        self.flip.set(dim);
        self.wall.canvas.classes().set("dim", dim);
    }

    fn set_pointer_over(&self, value: bool) {
        for tile in &self.wall.tiles {
            tile.text.set_pseudo_class(POINTER_OVER, value);
        }
    }

    /// The state before a measured switch: the pseudo-class of every text as `value`, laid out.
    fn prepare(&self, value: bool) {
        self.set_pointer_over(value);
        self.wall.fixture.lay_out();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling_subsystem", "Activation").min_iteration_time(200);
    let parameters = format!("Controls={CONTROLS}");
    class.benchmark("ancestor_class", parameters.clone(), Activation::ancestor_class, |b| b.toggle_ancestor_class());
    class.benchmark_with_iteration_setup(
        "pseudo_class_set",
        parameters.clone(),
        || Activation::pseudo_class(false, true),
        |b| b.prepare(false),
        |b| b.set_pointer_over(true),
    );
    class.benchmark_with_iteration_setup(
        "pseudo_class_reset",
        parameters.clone(),
        || Activation::pseudo_class(false, true),
        |b| b.prepare(true),
        |b| b.set_pointer_over(false),
    );
    class.benchmark_with_iteration_setup(
        "pseudo_class_set_unobserved",
        parameters.clone(),
        || Activation::pseudo_class(false, false),
        |b| b.prepare(false),
        |b| b.set_pointer_over(true),
    );
    class.benchmark_with_iteration_setup(
        "pseudo_class_set_over_style",
        parameters,
        || Activation::pseudo_class(true, true),
        |b| b.prepare(false),
        |b| b.set_pointer_over(true),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "Activation");
    }
}
