//! Children that are hidden and shown again: one child in ten of a panel of a thousand changes its
//! visibility, and the layout manager runs a pass. A hidden child desires nothing and tells its
//! parent so; a child shown again is measured again (section 9.9 of the performance notes: the
//! copy of the visibility a layoutable reads in its measure and its arrange; section 9.21: the
//! children that did not change are visited and left alone).
//!
//! - `stack_panel`: the children below a hidden one move up, so the panel arranges each of them
//!   at a new place;
//! - `wrap_panel`: the lines of the panel are filled again;
//! - `canvas`: no other child moves: the cost of the change itself and of a panel that visits
//!   its children to find nothing to do.
//!
//! Each invocation hides the children or shows them, in turn.

use super::fixture::{leaves, show};
use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::layout::Orientation;
use ferroui_base::Ref;
use ferroui_controls::{Border, Canvas, Control, StackPanel, WrapPanel};
use std::cell::Cell;

const CHILDREN: usize = 1_000;

/// One child in this many changes its visibility.
const CHANGED_STEP: usize = 10;

pub struct Visibility {
    fixture: Fixture,
    /// The children that are hidden and shown.
    changed: Vec<Ref<Border>>,
    visible: Cell<bool>,
}

impl Visibility {
    /// Shows `panel`, which holds `children`, and lays it out.
    fn shown(fixture: Fixture, panel: Ref<Control>, children: Vec<Ref<Border>>) -> Self {
        show(&fixture, &panel);
        let changed = children.into_iter().step_by(CHANGED_STEP).collect();
        Self { fixture, changed, visible: Cell::new(true) }
    }

    fn stack_panel() -> Self {
        let fixture = Fixture::new();
        let children = leaves(CHILDREN, 10.0, 4.0, &fixture.brushes);
        let panel = StackPanel::new();
        for child in &children {
            panel.children().add(child);
        }
        Self::shown(fixture, panel.upcast(), children)
    }

    fn wrap_panel() -> Self {
        let fixture = Fixture::new();
        let children = leaves(CHILDREN, 20.0, 10.0, &fixture.brushes);
        let panel = WrapPanel::new();
        panel.set_orientation(Orientation::Horizontal);
        for child in &children {
            panel.children().add(child);
        }
        Self::shown(fixture, panel.upcast(), children)
    }

    fn canvas() -> Self {
        let fixture = Fixture::new();
        let children = leaves(CHILDREN, 10.0, 4.0, &fixture.brushes);
        let panel = Canvas::new();
        for (index, child) in children.iter().enumerate() {
            Canvas::set_left(child, (index % 100) as f64 * 12.0);
            Canvas::set_top(child, (index / 100) as f64 * 6.0);
            panel.children().add(child);
        }
        Self::shown(fixture, panel.upcast(), children)
    }

    fn toggle(&mut self) {
        let visible = !self.visible.get();
        self.visible.set(visible);
        for child in &self.changed {
            child.set_is_visible(visible);
        }
        self.fixture.lay_out();
    }
}

type Setup = fn() -> Visibility;

const PANELS: [(&str, Setup); 3] =
    [("stack_panel", Visibility::stack_panel), ("wrap_panel", Visibility::wrap_panel), ("canvas", Visibility::canvas)];

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout_subsystem", "Visibility").min_iteration_time(200);
    let parameters = format!("Children={CHILDREN}");
    for (name, setup) in PANELS {
        class.benchmark(name, parameters.clone(), setup, Visibility::toggle);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "Visibility");
    }
}
