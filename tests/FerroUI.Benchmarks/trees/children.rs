//! Children added to a panel: one at a time and as one range, to a panel a root shows and to a
//! panel in no tree, at three sizes. The time is that of one child, so a cost that grows faster
//! than the number of children shows as a time that grows with the size (experiment #754).
//!
//! - `one_by_one`: `children().add(child)` for each child: the logical and the visual attachment
//!   of the child, the lists of the parent, the invalidation of the layout;
//! - `range`: `children().add_range(children)`: the same children in one change of the list.

use super::fixture::Fixture;
use crate::harness::Registry;
use ferroui_base::Ref;
use ferroui_controls::{Border, Control, StackPanel};

const SIZES: [usize; 3] = [64, 256, 1_024];

pub struct Children {
    fixture: Fixture,
    count: usize,
    attached: bool,
    panel: Option<Ref<StackPanel>>,
    pending: Vec<Ref<Border>>,
}

impl Children {
    fn new(count: usize, attached: bool) -> Self {
        Self { fixture: Fixture::new(), count, attached, panel: None, pending: Vec::new() }
    }

    /// An empty panel, shown in the root or in no tree, and the children that wait for it.
    fn prepare(&mut self) {
        self.panel = None;
        self.fixture.show(None);
        // The layout queue holds what was detached until its next pass: freed here, unmeasured.
        self.fixture.lay_out();
        let panel = StackPanel::new();
        if self.attached {
            self.fixture.show(Some(&panel.clone().upcast::<Control>()));
            self.fixture.lay_out();
        }
        self.panel = Some(panel);
        self.pending = (0..self.count)
            .map(|_| {
                let child = Border::new();
                child.set_width(40.0);
                child.set_height(20.0);
                child.set_background(Some(self.fixture.brushes.tile.clone()));
                child
            })
            .collect();
    }

    fn one_by_one(&mut self) {
        let panel = self.panel.clone().expect("a panel");
        for child in std::mem::take(&mut self.pending) {
            panel.children().add(child);
        }
    }

    fn range(&mut self) {
        let panel = self.panel.clone().expect("a panel");
        let children = std::mem::take(&mut self.pending);
        panel.children().add_range(children.into_iter().map(|child| child.upcast::<Control>()));
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("trees", "Children").min_iteration_time(200);
    for attached in [true, false] {
        for count in SIZES {
            let parameters = format!("Children={count}, Parent={}", if attached { "attached" } else { "detached" });
            class
                .benchmark_with_iteration_setup(
                    "one_by_one",
                    parameters.clone(),
                    move || Children::new(count, attached),
                    Children::prepare,
                    Children::one_by_one,
                )
                .operations_per_invoke(count as u32);
            class
                .benchmark_with_iteration_setup(
                    "range",
                    parameters,
                    move || Children::new(count, attached),
                    Children::prepare,
                    Children::range,
                )
                .operations_per_invoke(count as u32);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "Children");
    }
}
