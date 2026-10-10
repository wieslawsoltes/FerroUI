//! Builds the control trees of the layout, styling and traversal benchmarks.

use ferroui_base::Ref;
use ferroui_controls::{Button, Control, Panel, StackPanel};

pub struct ControlHierarchyCreator;

impl ControlHierarchyCreator {
    /// Adds `child_count` stack panels to `parent`, each followed by
    /// `inner_count` buttons of 100 by 50 that are children of `parent` as
    /// well, and does the same in every stack panel, `iterations` levels
    /// deep. Every control is added to `controls`: the buttons as they are
    /// created, a stack panel after the controls below it.
    pub fn create_children(
        mut controls: Vec<Ref<Control>>,
        parent: &Panel,
        child_count: i32,
        inner_count: i32,
        iterations: i32,
    ) -> Vec<Ref<Control>> {
        for _ in 0..child_count {
            let control = StackPanel::new();
            parent.children().add(&control);

            for _ in 0..inner_count {
                let child = Button::new();
                child.set_width(100.0);
                child.set_height(50.0);

                parent.children().add(&child);

                controls.push(child.upcast());
            }

            if iterations > 0 {
                controls = Self::create_children(controls, &control, child_count, inner_count, iterations - 1);
            }

            controls.push(control.upcast());
        }

        controls
    }
}
