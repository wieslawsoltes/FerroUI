// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

use super::{ElementRealizationOptions, ItemsRepeater, LayoutContextHost};
use ferroui_controls::Control;
use ferroui_base::{BoxedValue, Point, Rect, Ref, WeakRef};

/// Weak bridge from a reusable layout context to its owning repeater.
pub(crate) struct RepeaterLayoutContext(pub WeakRef<ItemsRepeater>);

impl RepeaterLayoutContext {
    fn owner(&self) -> Ref<ItemsRepeater> {
        self.0.upgrade().expect("The ItemsRepeater of this layout context was released.")
    }
}

impl LayoutContextHost for RepeaterLayoutContext {
    fn item_count(&self) -> i32 {
        self.0.upgrade().and_then(|owner| owner.items_source_view()).map_or(0, |items| items.count() as i32)
    }
    fn get_item_at(&self, index: i32) -> Option<BoxedValue> {
        let owner = self.owner();
        let items = owner.items_source_view().expect("ItemsSource doesn't have a value.");
        assert!(index >= 0 && (index as usize) < items.count(), "Argument index is invalid.");
        items.get_at(index as usize)
    }
    fn get_or_create_element_at(&self, index: i32, options: ElementRealizationOptions) -> Ref<Control> {
        self.owner().get_element_impl(index, options)
    }
    fn recycle_element(&self, element: &Ref<Control>) { self.owner().clear_element_impl(element); }
    fn layout_state(&self) -> Option<BoxedValue> {
        self.0.upgrade().and_then(|owner| owner.layout_state())
    }
    fn set_layout_state(&self, value: Option<BoxedValue>) { self.owner().set_layout_state(value); }
    fn realization_rect(&self) -> Rect { self.owner().realization_window() }
    fn visible_rect(&self) -> Rect { self.owner().visible_window() }
    fn recommended_anchor_index(&self) -> i32 {
        let owner = self.owner();
        owner.viewport_manager.suggested_anchor(&owner).map_or(-1, |anchor| owner.get_element_index(&anchor))
    }
    fn layout_origin(&self) -> Point { self.owner().layout_origin() }
    fn set_layout_origin(&self, value: Point) { self.owner().set_layout_origin(value); }
}
