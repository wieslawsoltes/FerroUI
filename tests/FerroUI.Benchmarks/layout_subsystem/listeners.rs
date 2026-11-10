//! The events a layout pass raises, on a canvas of a thousand leaves: the effective viewport of
//! an element, which the layout manager computes for every element with a handler at the end of
//! every pass and raises when it changed, and the layout updated event, raised for every element
//! with a handler at the end of every pass (section 9.28 of the performance notes: a pass does
//! not pay for the events nobody hears, and little for those that have nothing to say).
//!
//! - `idle`: a pass with nothing invalid and no handler: what the two below add to;
//! - `effective_viewport_idle`: the same pass with a handler of the effective viewport on every
//!   leaf: a viewport computed and compared for each, the viewports of their shared ancestors
//!   computed once, no event raised;
//! - `layout_updated_idle`: the same pass with a handler of the layout updated event on every
//!   leaf: a thousand events raised;
//! - `moved`: the canvas of the leaves moves on its parent and the layout manager runs a pass, no
//!   handler: what the one below adds to;
//! - `effective_viewport_moved`: the same with a handler of the effective viewport on every leaf:
//!   the viewport of each leaf changes, and a thousand events are raised.
//!
//! The port computes the transformed bounds of a visual on demand and keeps no listeners of them,
//! so there is nothing of them for a pass to do.

use super::fixture::{leaves, show};
use crate::harness::Registry;
use crate::trees::fixture::Fixture;
use ferroui_base::reactive::Subscription;
use ferroui_base::Ref;
use ferroui_controls::{Canvas, Control};
use std::cell::Cell;
use std::rc::Rc;

const LEAVES: usize = 1_000;

/// The event the leaves have a handler of.
#[derive(Clone, Copy, PartialEq)]
enum Heard {
    Nothing,
    EffectiveViewport,
    LayoutUpdated,
}

pub struct LayoutListeners {
    /// Declared first: the handlers are removed while their elements are in the tree.
    _subscriptions: Vec<Subscription>,
    fixture: Fixture,
    /// The canvas of the leaves, itself a child of a canvas.
    inner: Ref<Canvas>,
    flip: Cell<bool>,
}

impl LayoutListeners {
    fn new(heard: Heard) -> Self {
        let fixture = Fixture::new();
        let leaves = leaves(LEAVES, 10.0, 4.0, &fixture.brushes);
        let inner = Canvas::new();
        for (index, leaf) in leaves.iter().enumerate() {
            Canvas::set_left(leaf, (index % 100) as f64 * 12.0);
            Canvas::set_top(leaf, (index / 100) as f64 * 6.0);
            inner.children().add(leaf);
        }
        let outer = Canvas::new();
        outer.set_width(1280.0);
        outer.set_height(820.0);
        Canvas::set_left(&inner, 0.0);
        Canvas::set_top(&inner, 0.0);
        outer.children().add(&inner);
        show(&fixture, &outer.upcast::<Control>());

        // The handlers count their calls: the pass after they are added raises each event once
        // for each leaf, which shows that the handlers are heard.
        let raised = Rc::new(Cell::new(0_usize));
        let subscriptions: Vec<Subscription> = match heard {
            Heard::Nothing => Vec::new(),
            Heard::EffectiveViewport => leaves
                .iter()
                .map(|leaf| {
                    let raised = raised.clone();
                    leaf.effective_viewport_changed(move |_| raised.set(raised.get() + 1))
                })
                .collect(),
            Heard::LayoutUpdated => leaves
                .iter()
                .map(|leaf| {
                    let raised = raised.clone();
                    leaf.layout_updated(move || raised.set(raised.get() + 1))
                })
                .collect(),
        };
        fixture.lay_out();
        if heard != Heard::Nothing {
            assert_eq!(raised.get(), LEAVES, "the pass raises the event once for each leaf");
        }

        Self { _subscriptions: subscriptions, fixture, inner, flip: Cell::new(false) }
    }

    fn lay_out(&mut self) {
        self.fixture.lay_out();
    }

    fn move_leaves(&mut self) {
        let moved = !self.flip.get();
        self.flip.set(moved);
        Canvas::set_left(&self.inner, if moved { 8.0 } else { 0.0 });
        self.fixture.lay_out();
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout_subsystem", "LayoutListeners").min_iteration_time(200);
    let parameters = format!("Leaves={LEAVES}");
    class.benchmark("idle", parameters.clone(), || LayoutListeners::new(Heard::Nothing), LayoutListeners::lay_out);
    class.benchmark(
        "effective_viewport_idle",
        parameters.clone(),
        || LayoutListeners::new(Heard::EffectiveViewport),
        LayoutListeners::lay_out,
    );
    class.benchmark(
        "layout_updated_idle",
        parameters.clone(),
        || LayoutListeners::new(Heard::LayoutUpdated),
        LayoutListeners::lay_out,
    );
    class.benchmark("moved", parameters.clone(), || LayoutListeners::new(Heard::Nothing), LayoutListeners::move_leaves);
    class.benchmark(
        "effective_viewport_moved",
        parameters,
        || LayoutListeners::new(Heard::EffectiveViewport),
        LayoutListeners::move_leaves,
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        crate::harness::smoke_class(super::register, "LayoutListeners");
    }
}
