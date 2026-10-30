// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

use super::{
    FlowLayout, Layout, LayoutImpl, LayoutPanel, NonVirtualizingLayout,
    NonVirtualizingLayoutContext, NonVirtualizingLayoutImpl, StackLayout, UniformGridLayout,
};
use ferroui_controls::{Border, Control};
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Size};
use std::cell::Cell;
use std::rc::Rc;

#[derive(PartialEq)]
struct LayoutStateLifetime(Rc<Cell<usize>>);
impl Drop for LayoutStateLifetime {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[repr(C)]
struct RetainingStateLayout {
    base: NonVirtualizingLayout,
    released_states: Rc<Cell<usize>>,
    uninitialized_with_state: Cell<bool>,
}
ferro_class!(RetainingStateLayout: NonVirtualizingLayout);
ferro_impl_classes!(RetainingStateLayout: FerroObjectImpl, LayoutImpl);
impl NonVirtualizingLayoutImpl for RetainingStateLayout {
    fn initialize_for_context_core(this: &Self, context: &Ref<NonVirtualizingLayoutContext>) {
        assert!(context.layout_state().is_none());
        context.set_layout_state(Some(Rc::new(LayoutStateLifetime(
            this.released_states.clone(),
        ))));
    }

    fn uninitialize_for_context_core(this: &Self, context: &Ref<NonVirtualizingLayoutContext>) {
        this.uninitialized_with_state.set(
            context
                .layout_state()
                .is_some_and(|state| state.is::<LayoutStateLifetime>()),
        );
        // Deliberately retain the state: the panel owns the context lifetime.
    }

    fn measure_override(
        _this: &Self,
        context: &Ref<NonVirtualizingLayoutContext>,
        available: Size,
    ) -> Size {
        let mut desired = Size::default();
        for child in context.children() {
            child.measure(available);
            desired.width = desired.width.max(child.desired_size().width);
            desired.height = desired.height.max(child.desired_size().height);
        }
        desired
    }

    fn arrange_override(
        _this: &Self,
        context: &Ref<NonVirtualizingLayoutContext>,
        final_size: Size,
    ) -> Size {
        for child in context.children() {
            child.arrange(ferroui_base::Rect::from_size(final_size));
        }
        final_size
    }
}
impl RetainingStateLayout {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: NonVirtualizingLayout::construct(),
            released_states: Rc::new(Cell::new(0)),
            uninitialized_with_state: Cell::new(false),
        })
    }
}

#[test]
fn replacing_a_custom_panel_layout_releases_its_state_after_uninitialization() {
    for replacement in [
        Some(StackLayout::new().upcast::<Layout>()),
        Some(FlowLayout::new().upcast::<Layout>()),
        Some(UniformGridLayout::new().upcast::<Layout>()),
        Some(RetainingStateLayout::new().upcast::<Layout>()),
        None,
    ] {
        let panel = LayoutPanel::new();
        let child = Border::new();
        child.set_width(10.0);
        child.set_height(20.0);
        panel.children().add(&child.clone().upcast::<Control>());
        let original = RetainingStateLayout::new();
        panel.set_layout(Some(original.clone().upcast()));
        let retired = Rc::downgrade(&panel.layout_state().unwrap());

        panel.set_layout(replacement.clone());
        assert!(original.uninitialized_with_state.get());
        assert_eq!(original.released_states.get(), 1);
        assert!(retired.upgrade().is_none());
        assert_eq!(panel.layout_state().is_some(), replacement.is_some());
        panel.measure(Size::new(100.0, 100.0));
        panel.arrange(ferroui_base::Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(panel.children().count(), 1);
        assert!(panel.children().snapshot()[0].ptr_eq(&child.upcast::<Control>()));

        panel.set_layout(None);
        assert!(panel.layout_state().is_none());
    }
}
