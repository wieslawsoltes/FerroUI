//! A styled subtree taken out of an attached tree and put back: a panel that holds every tile of
//! the wall moves between two parents. A detached element gives up its styles, and an attached
//! one finds its theme, evaluates the styles in scope and applies them, whether or not the
//! styles it sees are those it had: a move between two parents with the same styles costs what a
//! move between parents with different styles costs. The styles set the `Tag` of the texts.
//! `trees::Lifetime` and `trees::Styling.attach` measure a new wall shown for the first time.
//!
//! - `move_unstyled`: the move with the global styles alone in scope, the baseline;
//! - `move_same_styles`: the root holds a style of the texts: the same style before and after;
//! - `move_other_styles`: each parent holds its own style of the texts: the style of the old
//!   parent is replaced with that of the new one;
//! - `detach`, `reattach`: the two halves of `move_same_styles`: the panel removed from its
//!   parent, and added to it again.

use super::wall::{tag, Wall, CONTROLS};
use crate::harness::Registry;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::Ref;
use ferroui_controls::{Control, Panel, TextBlock};
use std::cell::Cell;

/// The hosts of the styles that select the texts.
#[derive(Clone, Copy, PartialEq)]
enum StyleHosts {
    None,
    Root,
    Parents,
}

pub struct DetachReattach {
    wall: Wall,
    /// The panel that holds the tiles.
    section: Ref<Panel>,
    /// The two panels of the canvas the section moves between.
    parents: [Ref<Panel>; 2],
    /// The parent that holds the section, when one does.
    at: Cell<Option<usize>>,
}

impl DetachReattach {
    fn new(hosts: StyleHosts) -> Self {
        let wall = Wall::with_sections(CONTROLS, 1);
        let section = wall.sections[0].clone();
        wall.canvas.children().remove(&section);
        let parents = [Panel::new(), Panel::new()];
        for (index, parent) in parents.iter().enumerate() {
            if hosts == StyleHosts::Parents {
                parent.styles().add(&Self::style(index as i32 + 1));
            }
            wall.canvas.children().add(parent);
        }
        if hosts == StyleHosts::Root {
            wall.fixture.root.styles().add(&Self::style(1));
        }
        parents[0].children().add(&section);
        wall.attach();
        let mut this = Self { wall, section, parents, at: Cell::new(Some(0)) };
        this.expect(hosts, "after the attach");
        this.move_over();
        this.expect(hosts, "after the move");
        this.detach();
        if this.wall.tagged_texts() != 0 {
            panic!("Invalid benchmark state: {} detached texts keep a style", this.wall.tagged_texts());
        }
        this.reattach();
        this.expect(hosts, "after the panel is added again");
        this.wall.fixture.lay_out();
        this
    }

    /// A style that sets the `Tag` of the texts to `value`.
    fn style(value: i32) -> Ref<Style> {
        Style::with_setters(Selectors::of_type::<TextBlock>(), [Setter::new(Control::tag_property(), tag(value))])
    }

    /// Checks that every text has the value of the style in scope at the section's parent.
    fn expect(&self, hosts: StyleHosts, when: &str) {
        let at = self.at.get().expect("an attached section");
        let (found, texts) = match hosts {
            StyleHosts::None => (self.wall.tagged_texts(), 0),
            StyleHosts::Root => (self.wall.texts_with_tag(1), CONTROLS),
            StyleHosts::Parents => (self.wall.texts_with_tag(at as i32 + 1), CONTROLS),
        };
        if found != texts {
            panic!("Invalid benchmark state: {found} texts have the style in scope {when}, {texts} are expected");
        }
    }

    /// Removes the section from the parent that holds it.
    fn detach(&mut self) {
        if let Some(at) = self.at.take() {
            self.parents[at].children().remove(&self.section);
        }
    }

    /// Adds the detached section to the first parent.
    fn reattach(&mut self) {
        if self.at.get().is_none() {
            self.parents[0].children().add(&self.section);
            self.at.set(Some(0));
        }
    }

    /// Moves the section to the other parent.
    fn move_over(&mut self) {
        let at = self.at.get().expect("an attached section");
        self.parents[at].children().remove(&self.section);
        self.parents[1 - at].children().add(&self.section);
        self.at.set(Some(1 - at));
    }

    fn settle(&mut self) {
        self.wall.fixture.lay_out();
    }

    /// The state before a measured removal: the section is attached and laid out.
    fn prepare_detach(&mut self) {
        self.reattach();
        self.wall.fixture.lay_out();
    }

    /// The state before a measured addition: the section is detached.
    fn prepare_reattach(&mut self) {
        self.detach();
        self.wall.fixture.lay_out();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("styling_subsystem", "DetachReattach").min_iteration_time(200);
    let parameters = format!("Controls={CONTROLS}");
    class
        .benchmark_with_iteration_setup(
            "move_unstyled",
            parameters.clone(),
            || DetachReattach::new(StyleHosts::None),
            DetachReattach::settle,
            DetachReattach::move_over,
        )
        .baseline();
    class.benchmark_with_iteration_setup(
        "move_same_styles",
        parameters.clone(),
        || DetachReattach::new(StyleHosts::Root),
        DetachReattach::settle,
        DetachReattach::move_over,
    );
    class.benchmark_with_iteration_setup(
        "move_other_styles",
        parameters.clone(),
        || DetachReattach::new(StyleHosts::Parents),
        DetachReattach::settle,
        DetachReattach::move_over,
    );
    class.benchmark_with_iteration_setup(
        "detach",
        parameters.clone(),
        || DetachReattach::new(StyleHosts::Root),
        DetachReattach::prepare_detach,
        DetachReattach::detach,
    );
    class.benchmark_with_iteration_setup(
        "reattach",
        parameters,
        || DetachReattach::new(StyleHosts::Root),
        DetachReattach::prepare_reattach,
        DetachReattach::reattach,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "DetachReattach");
    }
}
