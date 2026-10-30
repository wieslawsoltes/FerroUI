// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use super::{
    ElementRealizationOptions, OrientationBasedMeasures, ScrollOrientation,
    VirtualizingLayoutContext,
};
use ferroui_controls::{Control, ItemsChangedEventArgs};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::{Rect, Ref, WeakRef};
use std::cell::{Cell, RefCell};

const INVALID_BOUNDS: Rect = Rect::new(-1.0, -1.0, -1.0, -1.0);
const REALIZE: ElementRealizationOptions =
    ElementRealizationOptions::FORCE_CREATE.union(ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE);

/// A contiguous realized range with lazy sentinels for collection insertions.
/// Per-element bounds stay independent of the final line alignment.
pub struct ElementManager {
    context: RefCell<Option<WeakRef<VirtualizingLayoutContext>>>,
    realized_elements: RefCell<Vec<Option<Ref<Control>>>>,
    layout_bounds: RefCell<Vec<Rect>>,
    first_realized_data_index: Cell<i32>,
}
impl Default for ElementManager {
    fn default() -> Self {
        Self::new()
    }
}
impl ElementManager {
    pub fn new() -> Self {
        Self {
            context: RefCell::new(None),
            realized_elements: RefCell::new(Vec::new()),
            layout_bounds: RefCell::new(Vec::new()),
            first_realized_data_index: Cell::new(-1),
        }
    }
    pub fn set_context(&self, context: &Ref<VirtualizingLayoutContext>) {
        *self.context.borrow_mut() = Some(context.downgrade());
    }
    fn context(&self) -> Ref<VirtualizingLayoutContext> {
        self.context
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
            .expect("ElementManager must have an initialized context")
    }
    fn is_virtualizing(&self) -> bool {
        self.context().is_virtualizing()
    }
    pub fn on_begin_measure(&self, orientation: ScrollOrientation) {
        if self.is_virtualizing() {
            self.discard_outside_rect(self.context().realization_rect(), orientation);
        } else {
            self.layout_bounds
                .borrow_mut()
                .resize(self.context().item_count().max(0) as usize, Rect::default());
        }
    }
    pub fn first_realized_data_index(&self) -> i32 {
        if self.is_virtualizing() {
            self.first_realized_data_index.get()
        } else {
            0
        }
    }
    pub fn realized_element_count(&self) -> i32 {
        if self.is_virtualizing() {
            self.realized_elements.borrow().len() as i32
        } else {
            self.context().item_count()
        }
    }
    pub fn data_index_from_range_index(&self, index: i32) -> i32 {
        if self.is_virtualizing() {
            index + self.first_realized_data_index.get()
        } else {
            index
        }
    }
    pub fn range_index_from_data_index(&self, index: i32) -> i32 {
        if self.is_virtualizing() {
            index - self.first_realized_data_index.get()
        } else {
            index
        }
    }
    pub fn is_index_valid_in_data(&self, index: i32) -> bool {
        index >= 0 && index < self.context().item_count()
    }
    pub fn is_data_index_realized(&self, index: i32) -> bool {
        let first = self.first_realized_data_index();
        let count = self.realized_element_count();
        count > 0 && index >= first && index < first + count
    }
    pub fn get_at(&self, range_index: i32) -> Ref<Control> {
        if !self.is_virtualizing() {
            return self
                .context()
                .get_or_create_element_at_with_options(range_index, REALIZE);
        }
        let existing = self.realized_elements.borrow()[range_index as usize].clone();
        if let Some(element) = existing {
            return element;
        }
        let element = self.context().get_or_create_element_at_with_options(
            self.data_index_from_range_index(range_index),
            REALIZE,
        );
        self.realized_elements.borrow_mut()[range_index as usize] = Some(element.clone());
        element
    }
    pub fn get_realized_element(&self, index: i32) -> Ref<Control> {
        self.get_at(self.range_index_from_data_index(index))
    }
    pub fn get_element_if_realized(&self, index: i32) -> Option<Ref<Control>> {
        self.is_data_index_realized(index)
            .then(|| self.get_realized_element(index))
    }
    pub fn add(&self, element: Ref<Control>, index: i32) {
        if self.realized_elements.borrow().is_empty() {
            self.first_realized_data_index.set(index);
        }
        self.realized_elements.borrow_mut().push(Some(element));
        self.layout_bounds.borrow_mut().push(Rect::default());
    }
    pub fn insert(&self, range_index: i32, data_index: i32, element: Option<Ref<Control>>) {
        if range_index == 0 {
            self.first_realized_data_index.set(data_index);
        }
        self.realized_elements
            .borrow_mut()
            .insert(range_index as usize, element);
        self.layout_bounds
            .borrow_mut()
            .insert(range_index as usize, INVALID_BOUNDS);
    }
    pub fn ensure_element_realized(&self, forward: bool, index: i32) {
        if !self.is_data_index_realized(index) {
            let element = self
                .context()
                .get_or_create_element_at_with_options(index, REALIZE);
            if forward {
                self.add(element, index);
            } else {
                self.insert(0, index, Some(element));
            }
        }
    }
    pub fn clear_realized_range(&self) {
        if self.is_virtualizing() {
            self.clear_range(0, self.realized_element_count());
        } else {
            self.layout_bounds.borrow_mut().clear();
        }
    }
    pub fn clear_range(&self, start: i32, count: i32) {
        if count <= 0 {
            return;
        }
        let removed: Vec<_> = self
            .realized_elements
            .borrow_mut()
            .drain(start as usize..(start + count) as usize)
            .flatten()
            .collect();
        self.layout_bounds
            .borrow_mut()
            .drain(start as usize..(start + count) as usize);
        if start == 0 {
            self.first_realized_data_index
                .set(if self.realized_elements.borrow().is_empty() {
                    -1
                } else {
                    self.first_realized_data_index.get() + count
                });
        }
        // Remove our references before host callbacks can reenter the manager.
        let context = self.context();
        if start == 0 {
            for element in removed {
                context.recycle_element(&element);
            }
        } else {
            for element in removed.into_iter().rev() {
                context.recycle_element(&element);
            }
        }
    }
    pub fn discard_elements_outside_window(&self, forward: bool, start_index: i32) {
        if self.is_virtualizing() && self.is_data_index_realized(start_index) {
            let index = self.range_index_from_data_index(start_index);
            if forward {
                self.clear_range(index, self.realized_element_count() - index);
            } else {
                self.clear_range(0, index + 1);
            }
        }
    }
    pub fn layout_bounds_for_data_index(&self, index: i32) -> Rect {
        self.layout_bounds_for_range_index(self.range_index_from_data_index(index))
    }
    pub fn set_layout_bounds_for_data_index(&self, index: i32, bounds: Rect) {
        self.set_layout_bounds_for_range_index(self.range_index_from_data_index(index), bounds);
    }
    pub fn layout_bounds_for_range_index(&self, index: i32) -> Rect {
        self.layout_bounds.borrow()[index as usize]
    }
    pub fn set_layout_bounds_for_range_index(&self, index: i32, bounds: Rect) {
        let mut entries = self.layout_bounds.borrow_mut();
        if entries.len() <= index as usize {
            entries.resize(index as usize + 1, Rect::default());
        }
        entries[index as usize] = bounds;
    }
    pub fn is_layout_bounds_set(&self, index: i32) -> bool {
        self.layout_bounds_for_range_index(index) != INVALID_BOUNDS
    }
    pub fn is_window_connected(
        &self,
        window: Rect,
        orientation: ScrollOrientation,
        scroll_same_as_flow: bool,
    ) -> bool {
        let count = self.realized_element_count();
        if count == 0 {
            return false;
        }
        let orientation = if scroll_same_as_flow {
            if orientation == ScrollOrientation::Vertical {
                ScrollOrientation::Horizontal
            } else {
                ScrollOrientation::Vertical
            }
        } else {
            orientation
        };
        let axis = OrientationBasedMeasures::new(orientation);
        axis.major_start(self.layout_bounds_for_range_index(0)) <= axis.major_end(window)
            && axis.major_end(self.layout_bounds_for_range_index(count - 1))
                >= axis.major_start(window)
    }
    fn discard_outside_rect(&self, window: Rect, orientation: ScrollOrientation) {
        let axis = OrientationBasedMeasures::new(orientation);
        let bounds = self.layout_bounds.borrow();
        let intersects = |rect: Rect| {
            axis.major_end(window) >= axis.major_start(rect)
                && axis.major_start(window) <= axis.major_end(rect)
        };
        let count = bounds.len() as i32;
        let front = bounds.iter().take_while(|rect| !intersects(**rect)).count() as i32 - 1;
        let back = count
            - bounds
                .iter()
                .rev()
                .take_while(|rect| !intersects(**rect))
                .count() as i32;
        drop(bounds);
        // Keep one extra edge element, including an off-screen bring-into-view anchor.
        if back < count - 1 {
            self.clear_range(back + 1, count - back - 1);
        }
        if front > 0 {
            self.clear_range(0, front.min(self.realized_element_count()));
        }
    }
    pub fn data_source_changed(&self, args: &ItemsChangedEventArgs<'_>) {
        if !self.is_virtualizing() || self.realized_element_count() == 0 {
            return;
        }
        match args.action {
            NotifyCollectionChangedAction::Add => {
                self.on_items_added(args.new_starting_index, args.new_items.len() as i32)
            }
            NotifyCollectionChangedAction::Remove => {
                self.on_items_removed(args.old_starting_index, args.old_items.len() as i32)
            }
            NotifyCollectionChangedAction::Replace => {
                let old = args.old_items.len() as i32;
                let new = args.new_items.len() as i32;
                if old == new
                    && args.old_starting_index == args.new_starting_index
                    && self.is_data_index_realized(args.old_starting_index)
                    && self.is_data_index_realized(args.old_starting_index + old - 1)
                {
                    let start = self.range_index_from_data_index(args.old_starting_index) as usize;
                    let removed: Vec<_> = {
                        let mut items = self.realized_elements.borrow_mut();
                        items[start..start + old as usize]
                            .iter_mut()
                            .filter_map(Option::take)
                            .collect()
                    };
                    for element in removed {
                        self.context().recycle_element(&element);
                    }
                } else {
                    self.on_items_removed(args.old_starting_index, old);
                    self.on_items_added(args.new_starting_index, new);
                }
            }
            NotifyCollectionChangedAction::Move => {
                let count = args.old_items.len().max(1) as i32;
                self.on_items_removed(args.old_starting_index, count);
                self.on_items_added(args.new_starting_index, count);
            }
            NotifyCollectionChangedAction::Reset => self.clear_realized_range(),
        }
    }
    fn on_items_added(&self, index: i32, count: i32) {
        let first = self.first_realized_data_index.get();
        let last = first + self.realized_element_count() - 1;
        if index >= first && index <= last {
            for i in 0..count {
                self.insert(index - first + i, index + i, None);
            }
        } else if index <= first && first != -1 {
            self.first_realized_data_index.set(first + count);
        }
    }
    fn on_items_removed(&self, index: i32, count: i32) {
        let first = self.first_realized_data_index.get();
        let last = first + self.realized_element_count() - 1;
        let start = first.max(index);
        let end = last.min(index + count - 1);
        if end >= start {
            self.clear_range(self.range_index_from_data_index(start), end - start + 1);
        }
        if index <= first && self.first_realized_data_index.get() != -1 {
            self.first_realized_data_index
                .set(self.first_realized_data_index.get() - count);
        }
    }
}
